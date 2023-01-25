use crate::engine::wasm_api::*;
use radix_engine_interface::api::static_invoke_api::SerializableInvocation;
use radix_engine_interface::api::types::CallTableInvocation;
use radix_engine_interface::api::types::ScryptoReceiver;
use radix_engine_interface::api::types::{
    FnIdentifier, LockHandle, RENodeId, ScryptoRENode, SubstateOffset,
};
use radix_engine_interface::api::{EngineActorApi, EngineSubstateApi, Invokable};
use radix_engine_interface::data::{scrypto_decode, scrypto_encode};
use sbor::rust::fmt::Debug;
use sbor::rust::vec::Vec;
use sbor::*;

#[derive(Debug, Categorize, Encode, Decode)]
pub enum EngineSubstateApiError {
    DecodeError(DecodeError),
}

pub struct ScryptoEnv;

impl ScryptoEnv {
    // Slightly different from EngineComponentApi::invoke_method

    pub fn invoke_method(
        &mut self,
        receiver: ScryptoReceiver,
        method_name: &str,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, EngineSubstateApiError> {
        let receiver = scrypto_encode(&receiver).unwrap();

        let return_data = copy_buffer(unsafe {
            invoke_method(
                receiver.as_ptr(),
                receiver.len(),
                method_name.as_ptr(),
                method_name.len(),
                args.as_ptr(),
                args.len(),
            )
        });

        Ok(return_data)
    }
}

impl<N: SerializableInvocation> Invokable<N, EngineSubstateApiError> for ScryptoEnv {
    fn invoke(&mut self, input: N) -> Result<N::Output, EngineSubstateApiError> {
        let invocation = scrypto_encode(&Into::<CallTableInvocation>::into(input)).unwrap();

        let return_data = copy_buffer(unsafe { invoke(invocation.as_ptr(), invocation.len()) });

        scrypto_decode(&return_data).map_err(EngineSubstateApiError::DecodeError)
    }
}

impl EngineSubstateApi<EngineSubstateApiError> for ScryptoEnv {
    fn sys_create_node(&mut self, node: ScryptoRENode) -> Result<RENodeId, EngineSubstateApiError> {
        let node = scrypto_encode(&node).unwrap();

        let node_id = copy_buffer(unsafe { create_node(node.as_ptr(), node.len()) });

        scrypto_decode(&node_id).map_err(EngineSubstateApiError::DecodeError)
    }

    fn sys_drop_node(&mut self, node_id: RENodeId) -> Result<(), EngineSubstateApiError> {
        let node_id = scrypto_encode(&node_id).unwrap();

        unsafe { drop_node(node_id.as_ptr(), node_id.len()) };

        Ok(())
    }

    fn sys_get_visible_nodes(&mut self) -> Result<Vec<RENodeId>, EngineSubstateApiError> {
        let node_ids = copy_buffer(unsafe { get_visible_nodes() });

        scrypto_decode(&node_ids).map_err(EngineSubstateApiError::DecodeError)
    }

    fn sys_lock_substate(
        &mut self,
        node_id: RENodeId,
        offset: SubstateOffset,
        mutable: bool,
    ) -> Result<LockHandle, EngineSubstateApiError> {
        let node_id = scrypto_encode(&node_id).unwrap();
        let offset = scrypto_encode(&offset).unwrap();

        let handle = unsafe {
            lock_substate(
                node_id.as_ptr(),
                node_id.len(),
                offset.as_ptr(),
                offset.len(),
                mutable,
            )
        };

        Ok(handle)
    }

    fn sys_read(&mut self, lock_handle: LockHandle) -> Result<Vec<u8>, EngineSubstateApiError> {
        let substate = copy_buffer(unsafe { read_substate(lock_handle) });

        Ok(substate)
    }

    fn sys_write(
        &mut self,
        lock_handle: LockHandle,
        buffer: Vec<u8>,
    ) -> Result<(), EngineSubstateApiError> {
        unsafe { write_substate(lock_handle, buffer.as_ptr(), buffer.len()) };

        Ok(())
    }

    fn sys_drop_lock(&mut self, lock_handle: LockHandle) -> Result<(), EngineSubstateApiError> {
        unsafe { unlock_substate(lock_handle) };

        Ok(())
    }
}

impl EngineActorApi<EngineSubstateApiError> for ScryptoEnv {
    fn fn_identifier(&mut self) -> Result<FnIdentifier, EngineSubstateApiError> {
        let actor = copy_buffer(unsafe { get_actor() });

        scrypto_decode(&actor).map_err(EngineSubstateApiError::DecodeError)
    }
}

#[macro_export]
macro_rules! scrypto_env_native_fn {
    ($($vis:vis $fn:ident $fn_name:ident ($($args:tt)*) -> $rtn:ty { $arg:expr })*) => {
        $(
            $vis $fn $fn_name ($($args)*) -> $rtn {
                let mut env = crate::engine::scrypto_env::ScryptoEnv;
                radix_engine_interface::api::Invokable::invoke(&mut env, $arg).unwrap()
            }
        )+
    };
}
