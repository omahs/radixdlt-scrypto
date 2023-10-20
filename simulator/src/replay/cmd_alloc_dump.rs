use super::ledger_transaction_execution::*;
use super::txn_reader::TxnReader;
use super::Error;
use crate::replay::ledger_transaction::PreparedLedgerTransactionInner;
use clap::Parser;
use flate2::read::GzDecoder;
use flume;
use radix_engine::types::*;
use radix_engine::vm::wasm::*;
use radix_engine::vm::ScryptoVm;
use radix_engine_interface::prelude::NetworkDefinition;
use radix_engine_profiling::info_alloc::*;
use radix_engine_store_interface::db_key_mapper::SpreadPrefixKeyMapper;
use radix_engine_store_interface::interface::CommittableSubstateDatabase;
use radix_engine_stores::rocks_db_with_merkle_tree::RocksDBWithMerkleTreeSubstateStore;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tar::Archive;
use transaction::prelude::IntentHash;
use transaction::prelude::TransactionHashBech32Encoder;

/// Run transactions in archive using RocksDB and dump memory allocations
#[derive(Parser, Debug)]
pub struct TxnAllocDump {
    /// Path to the source Node state manager database
    pub source: PathBuf,
    /// Path to a folder for storing state
    pub database_dir: PathBuf,
    /// Path to the output file
    pub output_file: PathBuf,

    /// The network to use, [mainnet | stokenet]
    #[clap(short, long)]
    pub network: Option<String>,
    /// The max version to execute
    #[clap(short, long)]
    pub max_version: Option<u64>,
}

impl TxnAllocDump {
    pub fn run(&self) -> Result<(), Error> {
        let network = match &self.network {
            Some(n) => NetworkDefinition::from_str(n).map_err(Error::ParseNetworkError)?,
            None => NetworkDefinition::mainnet(),
        };

        let cur_version = {
            let database = RocksDBWithMerkleTreeSubstateStore::standard(self.database_dir.clone());
            let cur_version = database.get_current_version();
            if cur_version >= self.max_version.unwrap_or(u64::MAX) {
                return Ok(());
            }
            cur_version
        };
        let to_version = self.max_version.clone();

        let start = std::time::Instant::now();
        let (tx, rx) = flume::bounded(10);

        // txn reader
        let mut txn_reader = if self.source.is_file() {
            let tar_gz = File::open(&self.source).map_err(Error::IOError)?;
            let tar = GzDecoder::new(tar_gz);
            let archive = Archive::new(tar);
            TxnReader::TransactionFile(archive)
        } else if self.source.is_dir() {
            TxnReader::StateManagerDatabaseDir(self.source.clone())
        } else {
            return Err(Error::InvalidTransactionSource);
        };
        let txn_read_thread_handle =
            thread::spawn(move || txn_reader.read(cur_version, to_version, tx));

        // txn executor
        let mut database = RocksDBWithMerkleTreeSubstateStore::standard(self.database_dir.clone());
        let exists = self.output_file.exists();
        let mut output = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(&self.output_file)
            .map_err(Error::IOError)?;
        if !exists {
            writeln!(
                output,
                "TXID,Execution Cost Units,Heap allocations sum,Heap current level,Heap peak memory",
            )
            .map_err(Error::IOError)?;
        }

        let txn_write_thread_handle = thread::spawn(move || {
            let scrypto_vm = ScryptoVm::<DefaultWasmEngine>::default();
            let iter = rx.iter();
            for tx_payload in iter {
                let prepared = prepare_ledger_transaction(&tx_payload);

                INFO_ALLOC.set_enable(true);
                INFO_ALLOC.reset_counters();

                let receipt = execute_prepared_ledger_transaction(
                    &database,
                    &scrypto_vm,
                    &network,
                    &prepared,
                );

                let (heap_allocations_sum, heap_current_level, heap_peak_memory) =
                    INFO_ALLOC.get_counters_value();
                INFO_ALLOC.set_enable(false);

                let execution_cost_units = receipt
                    .fee_summary()
                    .map(|x| x.total_execution_cost_units_consumed.clone());
                let database_updates = receipt
                    .into_state_updates()
                    .create_database_updates::<SpreadPrefixKeyMapper>();
                database.commit(&database_updates);
                if let PreparedLedgerTransactionInner::UserV1(tx) = prepared.inner {
                    writeln!(
                        output,
                        "{},{},{},{},{}",
                        TransactionHashBech32Encoder::new(&network)
                            .encode(&IntentHash(tx.signed_intent.intent.summary.hash))
                            .unwrap(),
                        execution_cost_units.unwrap(),
                        heap_allocations_sum,
                        heap_current_level,
                        heap_peak_memory
                    )
                    .map_err(Error::IOError)?;
                }

                let new_state_root_hash = database.get_current_root_hash();
                let new_version = database.get_current_version();

                if new_version < 1000 || new_version % 1000 == 0 {
                    print_progress(start.elapsed(), new_version, new_state_root_hash);
                }
            }

            let duration = start.elapsed();
            println!("Time elapsed: {:?}", duration);
            println!("State version: {}", database.get_current_version());
            println!("State root hash: {}", database.get_current_root_hash());
            Ok::<(), Error>(())
        });

        txn_read_thread_handle.join().unwrap()?;
        txn_write_thread_handle.join().unwrap()?;

        Ok(())
    }
}

fn print_progress(duration: Duration, new_version: u64, new_root: Hash) {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() / 60) % 60;
    let hours = (duration.as_secs() / 60) / 60;
    println!(
        "New version: {}, {}, {:0>2}:{:0>2}:{:0>2}",
        new_version, new_root, hours, minutes, seconds
    );
}
