use crate::engine::wasm_api::*;
use radix_engine_interface::api::{types::*, ClientEventApi, ClientLoggerApi, LockFlags};
use radix_engine_interface::api::{
    ClientActorApi, ClientNodeApi, ClientObjectApi, ClientPackageApi, ClientSubstateApi,
};
use radix_engine_interface::blueprints::logger::Level;
use radix_engine_interface::blueprints::resource::AccessRulesConfig;
use radix_engine_interface::crypto::Hash;
use radix_engine_interface::data::scrypto::model::{Address, PackageAddress};
use radix_engine_interface::data::scrypto::*;
use radix_engine_interface::*;
use sbor::rust::prelude::*;
use sbor::*;
use scrypto_schema::PackageSchema;

#[derive(Debug, Sbor)]
pub enum ClientApiError {
    DecodeError(DecodeError),
}

pub struct ScryptoEnv;

impl ClientObjectApi<ClientApiError> for ScryptoEnv {
    fn new_object(
        &mut self,
        blueprint_ident: &str,
        app_states: Vec<Vec<u8>>,
    ) -> Result<ObjectId, ClientApiError> {
        let app_states = scrypto_encode(&app_states).unwrap();

        let bytes = copy_buffer(unsafe {
            new_component(
                blueprint_ident.as_ptr(),
                blueprint_ident.len(),
                app_states.as_ptr(),
                app_states.len(),
            )
        });
        scrypto_decode(&bytes).map_err(ClientApiError::DecodeError)
    }

    fn globalize(
        &mut self,
        node_id: RENodeId,
        modules: BTreeMap<NodeModuleId, ObjectId>,
    ) -> Result<Address, ClientApiError> {
        let node_id = scrypto_encode(&node_id).unwrap();
        let modules = scrypto_encode(&modules).unwrap();

        let bytes = copy_buffer(unsafe {
            globalize_component(
                node_id.as_ptr(),
                node_id.len(),
                modules.as_ptr(),
                modules.len(),
            )
        });
        scrypto_decode(&bytes).map_err(ClientApiError::DecodeError)
    }

    fn globalize_with_address(
        &mut self,
        node_id: RENodeId,
        modules: BTreeMap<NodeModuleId, ObjectId>,
        address: Address,
    ) -> Result<Address, ClientApiError> {
        let node_id = scrypto_encode(&node_id).unwrap();
        let modules = scrypto_encode(&modules).unwrap();
        let address = scrypto_encode(&address).unwrap();

        let bytes = copy_buffer(unsafe {
            globalize_with_address(
                node_id.as_ptr(),
                node_id.len(),
                modules.as_ptr(),
                modules.len(),
                address.as_ptr(),
                address.len(),
            )
        });
        scrypto_decode(&bytes).map_err(ClientApiError::DecodeError)
    }

    fn call_method(
        &mut self,
        receiver: RENodeId,
        method_name: &str,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, ClientApiError> {
        self.call_module_method(receiver, NodeModuleId::SELF, method_name, args)
    }

    fn call_module_method(
        &mut self,
        receiver: RENodeId,
        node_module_id: NodeModuleId,
        method_name: &str,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, ClientApiError> {
        let receiver = scrypto_encode(&receiver).unwrap();

        let return_data = copy_buffer(unsafe {
            call_method(
                receiver.as_ptr(),
                receiver.len(),
                node_module_id.id(),
                method_name.as_ptr(),
                method_name.len(),
                args.as_ptr(),
                args.len(),
            )
        });

        Ok(return_data)
    }

    fn get_object_type_info(
        &mut self,
        node_id: RENodeId,
    ) -> Result<(PackageAddress, String), ClientApiError> {
        let node_id = scrypto_encode(&node_id).unwrap();

        let bytes =
            copy_buffer(unsafe { get_component_type_info(node_id.as_ptr(), node_id.len()) });

        scrypto_decode(&bytes).map_err(ClientApiError::DecodeError)
    }

    fn new_key_value_store(&mut self) -> Result<KeyValueStoreId, ClientApiError> {
        let bytes = copy_buffer(unsafe { new_key_value_store() });
        scrypto_decode(&bytes).map_err(ClientApiError::DecodeError)
    }
}

impl ClientPackageApi<ClientApiError> for ScryptoEnv {
    fn new_package(
        &mut self,
        code: Vec<u8>,
        schema: PackageSchema,
        access_rules: AccessRulesConfig,
        royalty_config: BTreeMap<String, RoyaltyConfig>,
        metadata: BTreeMap<String, String>,
    ) -> Result<PackageAddress, ClientApiError> {
        let schema = scrypto_encode(&schema).unwrap();
        let access_rules = scrypto_encode(&access_rules).unwrap();
        let royalty_config = scrypto_encode(&royalty_config).unwrap();
        let metadata = scrypto_encode(&metadata).unwrap();

        let bytes = copy_buffer(unsafe {
            new_package(
                code.as_ptr(),
                code.len(),
                schema.as_ptr(),
                schema.len(),
                access_rules.as_ptr(),
                access_rules.len(),
                royalty_config.as_ptr(),
                royalty_config.len(),
                metadata.as_ptr(),
                metadata.len(),
            )
        });
        scrypto_decode(&bytes).map_err(ClientApiError::DecodeError)
    }

    fn call_function(
        &mut self,
        package_address: PackageAddress,
        blueprint_name: &str,
        function_name: &str,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, ClientApiError> {
        let package_address = scrypto_encode(&package_address).unwrap();

        let return_data = copy_buffer(unsafe {
            call_function(
                package_address.as_ptr(),
                package_address.len(),
                blueprint_name.as_ptr(),
                blueprint_name.len(),
                function_name.as_ptr(),
                function_name.len(),
                args.as_ptr(),
                args.len(),
            )
        });

        Ok(return_data)
    }
}

impl ClientNodeApi<ClientApiError> for ScryptoEnv {
    fn sys_drop_node(&mut self, node_id: RENodeId) -> Result<(), ClientApiError> {
        let node_id = scrypto_encode(&node_id).unwrap();

        unsafe { drop_node(node_id.as_ptr(), node_id.len()) };

        Ok(())
    }
}

impl ClientSubstateApi<ClientApiError> for ScryptoEnv {
    fn sys_lock_substate(
        &mut self,
        node_id: RENodeId,
        offset: SubstateOffset,
        flags: LockFlags,
    ) -> Result<LockHandle, ClientApiError> {
        let node_id = scrypto_encode(&node_id).unwrap();
        let offset = scrypto_encode(&offset).unwrap();

        let handle = unsafe {
            lock_substate(
                node_id.as_ptr(),
                node_id.len(),
                offset.as_ptr(),
                offset.len(),
                flags.bits(),
            )
        };

        Ok(handle)
    }

    fn sys_read_substate(&mut self, lock_handle: LockHandle) -> Result<Vec<u8>, ClientApiError> {
        let substate = copy_buffer(unsafe { read_substate(lock_handle) });

        Ok(substate)
    }

    fn sys_write_substate(
        &mut self,
        lock_handle: LockHandle,
        buffer: Vec<u8>,
    ) -> Result<(), ClientApiError> {
        unsafe { write_substate(lock_handle, buffer.as_ptr(), buffer.len()) };

        Ok(())
    }

    fn sys_drop_lock(&mut self, lock_handle: LockHandle) -> Result<(), ClientApiError> {
        unsafe { drop_lock(lock_handle) };

        Ok(())
    }
}

impl ClientActorApi<ClientApiError> for ScryptoEnv {
    fn get_fn_identifier(&mut self) -> Result<FnIdentifier, ClientApiError> {
        let actor = copy_buffer(unsafe { get_actor() });

        scrypto_decode(&actor).map_err(ClientApiError::DecodeError)
    }
}

impl ClientEventApi<ClientApiError> for ScryptoEnv {
    fn emit_raw_event(
        &mut self,
        schema_hash: Hash,
        event_data: Vec<u8>,
    ) -> Result<(), ClientApiError> {
        unsafe {
            emit_event(
                schema_hash.0.as_ptr(),
                schema_hash.0.len(),
                event_data.as_ptr(),
                event_data.len(),
            )
        };
        Ok(())
    }
}

impl ClientLoggerApi<ClientApiError> for ScryptoEnv {
    fn log_message(&mut self, level: Level, message: String) -> Result<(), ClientApiError> {
        let level = scrypto_encode(&level).unwrap();
        let message = scrypto_encode(&message).unwrap();

        unsafe { log_message(level.as_ptr(), level.len(), message.as_ptr(), message.len()) }
        Ok(())
    }
}

#[macro_export]
macro_rules! scrypto_env_native_fn {
    ($($vis:vis $fn:ident $fn_name:ident ($($args:tt)*) -> $rtn:ty { $arg:expr })*) => {
        $(
            $vis $fn $fn_name ($($args)*) -> $rtn {
                let mut env = crate::engine::scrypto_env::ScryptoEnv;
                env.call_native($arg).unwrap()
            }
        )+
    };
}
