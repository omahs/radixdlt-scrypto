use scrypto::api::substate_lock_api::LockFlags;
use scrypto::api::*;
use scrypto::engine::scrypto_env::*;
use scrypto::prelude::*;

#[blueprint]
mod node_create {
    struct NodeCreate {}

    impl NodeCreate {
        pub fn create_node_with_invalid_blueprint() {
            ScryptoEnv
                .new_object(
                    "invalid_blueprint",
                    vec![scrypto_encode(&NodeCreate {}).unwrap()],
                )
                .unwrap();
        }
    }
}
