use scrypto::prelude::*;

blueprint! {
    // nobody can instantiate a system component except the bootstrap process
    struct System {
        xrd: Vault,
    }

    impl System {
        /// Publishes a package.
        pub fn publish_package(code: Vec<u8>) -> Address {
            let package = Package::new(&code);
            package.into()
        }

        /// Creates a resource with mutable supply, and returns the resource definition address.
        pub fn new_resource_mutable(
            resource_type: ResourceType,
            metadata: HashMap<String, String>,
            auth_configs: ResourceAuthConfigs,
        ) -> Address {
            ResourceDef::new_mutable(resource_type, metadata, auth_configs).address()
        }

        /// Creates a resource with fixed supply, and returns all supply.
        pub fn new_resource_fixed(
            resource_type: ResourceType,
            metadata: HashMap<String, String>,
            supply: NewSupply,
        ) -> (Address, Bucket) {
            let (resource_def, bucket) = ResourceDef::new_fixed(resource_type, metadata, supply);
            (resource_def.address(), bucket)
        }

        /// Mints fungible resource.
        pub fn mint(amount: Decimal, resource_address: Address, auth: BucketRef) -> Bucket {
            ResourceDef::from(resource_address).mint(amount, auth)
        }

        /// Gives away XRD tokens for testing.
        pub fn free_xrd(&self, amount: Decimal) -> Bucket {
            self.xrd.take(amount)
        }
    }
}
