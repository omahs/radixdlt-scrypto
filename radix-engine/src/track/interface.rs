use crate::errors::RuntimeError;
use crate::types::*;
use radix_engine_interface::api::LockFlags;
use radix_engine_interface::types::*;
use radix_engine_store_interface::db_key_mapper::SubstateKeyContent;

/// Error when acquiring a lock.
#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum TrackGetSubstateError {
    NotFound(NodeId, PartitionNumber, SubstateKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackError<E, C> {
    Error(E),
    CallbackError(C),
}

impl<E> CallbackError<E, RuntimeError> {
    pub fn to_runtime_error<F: FnOnce(E) -> RuntimeError>(self, f: F) -> RuntimeError {
        match self {
            CallbackError::Error(e) => f(e),
            CallbackError::CallbackError(c) => c,
        }
    }
}

impl<E, C> CallbackError<E, C> {
    pub fn map<N, F: FnOnce(E) -> N>(self, f: F) -> CallbackError<N, C> {
        match self {
            CallbackError::Error(e) => CallbackError::Error(f(e)),
            CallbackError::CallbackError(c) => CallbackError::CallbackError(c),
        }
    }
}

pub type NodeSubstates = BTreeMap<PartitionNumber, BTreeMap<SubstateKey, IndexedScryptoValue>>;

pub enum TrackedSubstateInfo {
    New,
    Updated,
    Unmodified,
}

/// Represents the interface between Radix Engine and Track.
///
/// In practice, we will likely end up with only one implementation.
///
/// The trait here is for formalizing the interface and intended user flow.
pub trait SubstateStore {
    /// Inserts a node into the substate store.
    ///
    /// Clients must ensure the `node_id` is new and unique; otherwise, the behavior is undefined.
    ///
    /// # Panics
    /// - If the partition is invalid
    fn create_node<E, F: FnMut(StoreAccess) -> Result<(), E>>(
        &mut self,
        node_id: NodeId,
        node_substates: NodeSubstates,
        on_store_access: &mut F,
    ) -> Result<(), E>;

    fn get_tracked_substate_info(
        &mut self,
        node_id: &NodeId,
        partition_num: PartitionNumber,
        substate_key: &SubstateKey,
    ) -> TrackedSubstateInfo;

    /// Acquires a lock over a substate.
    /// Returns tuple of lock handle id and information if particular substate
    /// is locked for the first time during transaction execution.
    fn get_substate<E, F: FnMut(StoreAccess) -> Result<(), E>>(
        &mut self,
        node_id: &NodeId,
        partition_num: PartitionNumber,
        substate_key: &SubstateKey,
        on_store_access: F,
    ) -> Result<&IndexedScryptoValue, CallbackError<TrackGetSubstateError, E>> {
        self.get_substate_or_default(
            node_id,
            partition_num,
            substate_key,
            on_store_access,
            || None,
        )
    }

    fn get_substate_or_default<
        E,
        F: FnMut(StoreAccess) -> Result<(), E>,
        V: FnOnce() -> Option<IndexedScryptoValue>,
    >(
        &mut self,
        node_id: &NodeId,
        partition_num: PartitionNumber,
        substate_key: &SubstateKey,
        on_store_access: F,
        virtualize: V,
    ) -> Result<&IndexedScryptoValue, CallbackError<TrackGetSubstateError, E>>;

    /// Inserts a substate into the substate store.
    ///
    /// Clients must ensure the `node_id`/`partition_num` is a node which has been created; otherwise, the behavior
    /// is undefined.
    fn set_substate<E, F: FnMut(StoreAccess) -> Result<(), E>>(
        &mut self,
        node_id: NodeId,
        partition_num: PartitionNumber,
        substate_key: SubstateKey,
        substate_value: IndexedScryptoValue,
        on_store_access: &mut F,
    ) -> Result<(), E>;

    fn force_write(
        &mut self,
        node_id: &NodeId,
        partition_num: &PartitionNumber,
        substate_key: &SubstateKey,
    );

    /// Deletes a substate from the substate store.
    ///
    /// Clients must ensure the `node_id`/`partition_num` is a node which has been created;
    /// Clients must ensure this isn't called on a virtualized partition;
    /// Otherwise, the behavior is undefined.
    ///
    /// Returns tuple of substate and boolean which is true for the first database access.
    fn remove_substate<E, F: FnMut(StoreAccess) -> Result<(), E>>(
        &mut self,
        node_id: &NodeId,
        partition_num: PartitionNumber,
        substate_key: &SubstateKey,
        on_store_access: F,
    ) -> Result<Option<IndexedScryptoValue>, E>;

    /// Returns Substate Keys of maximum count for a given partition.
    ///
    /// Clients must ensure that the SubstateKeyContent which the partition is
    /// associated with is passed in. The returned SubstateKeys are guaranteed to be of
    /// this type.
    /// Otherwise, behavior is undefined.
    ///
    /// Returns list of substate keys and database access info
    fn scan_keys<K: SubstateKeyContent, E, F: FnMut(StoreAccess) -> Result<(), E>>(
        &mut self,
        node_id: &NodeId,
        partition_num: PartitionNumber,
        count: u32,
        on_store_access: F,
    ) -> Result<Vec<SubstateKey>, E>;

    /// Removes substates of maximum count for a given partition.
    ///
    /// Clients must ensure that the SubstateKeyContent which the partition is
    /// associated with is passed in. The returned SubstateKeys are guaranteed to be of
    /// this type.
    /// Otherwise, behavior is undefined.
    ///
    /// Returns list of removed substates with their associated keys and values, as well as database access info
    fn drain_substates<K: SubstateKeyContent, E, F: FnMut(StoreAccess) -> Result<(), E>>(
        &mut self,
        node_id: &NodeId,
        partition_num: PartitionNumber,
        count: u32,
        on_store_access: F,
    ) -> Result<Vec<(SubstateKey, IndexedScryptoValue)>, E>;

    /// Returns tuple of substate vector and boolean which is true for the first database access.
    fn scan_sorted_substates<E, F: FnMut(StoreAccess) -> Result<(), E>>(
        &mut self,
        node_id: &NodeId,
        partition_num: PartitionNumber,
        count: u32,
        on_store_access: F,
    ) -> Result<Vec<IndexedScryptoValue>, E>;

    /// Note: unstable interface, for intent transaction tracker only
    fn delete_partition(&mut self, node_id: &NodeId, partition_num: PartitionNumber);

    /// Return the commit info
    fn get_commit_info(&mut self) -> StoreCommitInfo;
}

#[derive(Debug, Clone, Copy)]
pub enum StoreAccess {
    /// Some substate was read from database.
    ReadFromDb(usize),
    /// Non-existent substate was read from database.
    ReadFromDbNotFound,
    /// A new entry has been added to track
    /// System limits how many items that can be tracked.
    NewEntryInTrack,
}

pub type StoreCommitInfo = Vec<StoreCommit>;

#[derive(Debug, Clone)]
pub enum StoreCommit {
    Insert {
        node_id: NodeId,
        size: usize,
    },
    Update {
        node_id: NodeId,
        size: usize,
        old_size: usize,
    },
    Delete {
        node_id: NodeId,
        old_size: usize,
    },
}

impl StoreCommit {
    pub fn node_id(&self) -> &NodeId {
        match self {
            StoreCommit::Insert { node_id, .. }
            | StoreCommit::Update { node_id, .. }
            | StoreCommit::Delete { node_id, .. } => node_id,
        }
    }
}
