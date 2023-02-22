use scrypto::engine::scrypto_env::*;
use scrypto::prelude::*;
use scrypto::radix_engine_interface::api::ClientSubstateApi;

#[blueprint]
mod caller {
    struct Caller;

    impl Caller {
        pub fn recursive(n: u32) {
            if n > 1 {
                let _: () = Runtime::call_function(
                    Runtime::package_address(),
                    "Caller",
                    "recursive",
                    scrypto_args!(n - 1),
                );
            }
        }
    }
}
