use crate::engine::*;
use crate::fee::FeeReserve;
use crate::model::*;
use crate::types::*;

// TODO: still lots of unwraps

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RENodePointer {
    Heap {
        frame_id: usize,
        root: RENodeId,
        id: Option<RENodeId>,
    },
    Store(RENodeId),
}

impl RENodePointer {
    pub fn acquire_lock<'s, R: FeeReserve>(
        &self,
        substate_id: SubstateId,
        mutable: bool,
        write_through: bool,
        track: &mut Track<'s, R>,
    ) -> Result<(), KernelError> {
        match self {
            RENodePointer::Store(..) => track
                .acquire_lock(substate_id.clone(), mutable, write_through)
                .map_err(KernelError::SubstateError),
            RENodePointer::Heap { .. } => Ok(()),
        }
    }

    pub fn release_lock<'s, R: FeeReserve>(
        &self,
        substate_id: SubstateId,
        write_through: bool,
        track: &mut Track<'s, R>,
    ) -> Result<(), KernelError> {
        match self {
            RENodePointer::Store(..) => track
                .release_lock(substate_id, write_through)
                .map_err(KernelError::SubstateError),
            RENodePointer::Heap { .. } => Ok(()),
        }
    }

    pub fn child(&self, child_id: RENodeId) -> RENodePointer {
        match self {
            RENodePointer::Heap { frame_id, root, .. } => RENodePointer::Heap {
                frame_id: frame_id.clone(),
                root: root.clone(),
                id: Option::Some(child_id),
            },
            RENodePointer::Store(..) => RENodePointer::Store(child_id),
        }
    }

    pub fn borrow_native_ref<'p, 's, R: FeeReserve>(
        &self, // TODO: Consider changing this to self
        substate_id: SubstateId,
        call_frames: &mut Vec<CallFrame>,
        track: &mut Track<'s, R>,
    ) -> NativeSubstateRef {
        match self {
            RENodePointer::Heap { frame_id, root, id } => {
                let frame = call_frames.get_mut(*frame_id).unwrap();
                let re_value = frame.owned_heap_nodes.remove(root).unwrap();
                NativeSubstateRef::Stack(re_value, frame_id.clone(), root.clone(), id.clone())
            }
            RENodePointer::Store(..) => {
                let value = track.take_substate(substate_id.clone());
                NativeSubstateRef::Track(substate_id.clone(), value)
            }
        }
    }

    pub fn to_ref<'f, 'p, 's, R: FeeReserve>(
        &self,
        call_frames: &'f Vec<CallFrame>,
        track: &'f mut Track<'s, R>,
    ) -> RENodeRef<'f, 's, R> {
        match self {
            RENodePointer::Heap { frame_id, root, id } => {
                let frame = call_frames.get(*frame_id).unwrap();
                RENodeRef::Stack(frame.owned_heap_nodes.get(root).unwrap(), id.clone())
            }
            RENodePointer::Store(node_id) => RENodeRef::Track(track, node_id.clone()),
        }
    }

    pub fn to_ref_mut<'f, 'p, 's, R: FeeReserve>(
        &self,
        call_frames: &'f mut Vec<CallFrame>,
        track: &'f mut Track<'s, R>,
    ) -> RENodeRefMut<'f, 's, R> {
        match self {
            RENodePointer::Heap { frame_id, root, id } => {
                let frame = call_frames.get_mut(*frame_id).unwrap();
                RENodeRefMut::Stack(frame.owned_heap_nodes.get_mut(root).unwrap(), id.clone())
            }
            RENodePointer::Store(node_id) => RENodeRefMut::Track(track, node_id.clone()),
        }
    }
}

#[derive(Debug)]
pub enum NativeSubstateRef {
    Stack(HeapRootRENode, usize, RENodeId, Option<RENodeId>),
    Track(SubstateId, BorrowedSubstate),
}

impl NativeSubstateRef {
    pub fn bucket_mut(&mut self) -> &mut Bucket {
        match self {
            NativeSubstateRef::Stack(root, _frame_id, _root_id, maybe_child) => {
                match root.get_node_mut(maybe_child.as_ref()) {
                    HeapRENode::Bucket(bucket) => bucket,
                    _ => panic!("Expecting to be a bucket"),
                }
            }
            _ => panic!("Expecting to be a bucket"),
        }
    }

    pub fn proof_mut(&mut self) -> &mut Proof {
        match self {
            NativeSubstateRef::Stack(ref mut root, _frame_id, _root_id, maybe_child) => {
                match root.get_node_mut(maybe_child.as_ref()) {
                    HeapRENode::Proof(proof) => proof,
                    _ => panic!("Expecting to be a proof"),
                }
            }
            _ => panic!("Expecting to be a proof"),
        }
    }

    pub fn worktop_mut(&mut self) -> &mut Worktop {
        match self {
            NativeSubstateRef::Stack(ref mut root, _frame_id, _root_id, maybe_child) => {
                match root.get_node_mut(maybe_child.as_ref()) {
                    HeapRENode::Worktop(worktop) => worktop,
                    _ => panic!("Expecting to be a worktop"),
                }
            }
            _ => panic!("Expecting to be a worktop"),
        }
    }

    pub fn vault_mut(&mut self) -> &mut Vault {
        match self {
            NativeSubstateRef::Stack(root, _frame_id, _root_id, maybe_child) => {
                root.get_node_mut(maybe_child.as_ref()).vault_mut()
            }
            NativeSubstateRef::Track(_address, value) => {
                value.substate.convert_to_node();
                value.substate.vault_mut()
            }
        }
    }

    pub fn system_mut(&mut self) -> &mut System {
        match self {
            NativeSubstateRef::Track(_address, value) => value.substate.raw_mut().system_mut(),
            _ => panic!("Expecting to be system"),
        }
    }

    pub fn component_info_mut(&mut self) -> &mut ComponentInfo {
        match self {
            NativeSubstateRef::Stack(root, _frame_id, _root_id, maybe_child) => {
                root.get_node_mut(maybe_child.as_ref()).component_info_mut()
            }
            _ => panic!("Expecting to be a component"),
        }
    }

    pub fn package_mut(&mut self) -> &Package {
        match self {
            NativeSubstateRef::Track(_address, value) => value.substate.raw().package(),
            _ => panic!("Expecting to be tracked"),
        }
    }

    pub fn resource_manager_mut(&mut self) -> &mut ResourceManager {
        match self {
            NativeSubstateRef::Stack(value, _frame_id, _root_id, maybe_child) => value
                .get_node_mut(maybe_child.as_ref())
                .resource_manager_mut(),
            NativeSubstateRef::Track(_address, value) => {
                value.substate.raw_mut().resource_manager_mut()
            }
        }
    }

    pub fn auth_zone_mut(&mut self) -> &mut AuthZone {
        match self {
            NativeSubstateRef::Stack(value, _frame_id, _root_id, maybe_child) => {
                value.get_node_mut(maybe_child.as_ref()).auth_zone_mut()
            }
            NativeSubstateRef::Track(..) => {
                panic!("Expecting not to be tracked.");
            }
        }
    }

    pub fn global_re_node(&mut self) -> &GlobalRENode {
        match self {
            NativeSubstateRef::Stack(..) => {
                panic!("Expecting not to be on stack.");
            }
            NativeSubstateRef::Track(_address, value) => value.substate.raw().global_re_node(),
        }
    }

    pub fn return_to_location<'a, 'p, 's, R: FeeReserve>(
        self,
        call_frames: &mut Vec<CallFrame>,
        track: &mut Track<'s, R>,
    ) {
        match self {
            NativeSubstateRef::Stack(owned, frame_id, node_id, ..) => {
                let frame = call_frames.get_mut(frame_id).unwrap();
                frame.owned_heap_nodes.insert(node_id, owned);
            }
            NativeSubstateRef::Track(substate_id, substate) => {
                track.return_substate(substate_id, substate)
            }
        }
    }
}
pub enum RENodeRef<'f, 's, R: FeeReserve> {
    Stack(&'f HeapRootRENode, Option<RENodeId>),
    Track(&'f mut Track<'s, R>, RENodeId),
}

impl<'f, 's, R: FeeReserve> RENodeRef<'f, 's, R> {
    pub fn bucket(&self) -> &Bucket {
        match self {
            RENodeRef::Stack(value, id) => id
                .as_ref()
                .map_or(value.root(), |v| value.non_root(v))
                .bucket(),
            RENodeRef::Track(..) => {
                panic!("Unexpected")
            }
        }
    }

    pub fn vault(&mut self) -> &Vault {
        match self {
            RENodeRef::Stack(value, id) => id
                .as_ref()
                .map_or(value.root(), |v| value.non_root(v))
                .vault(),

            RENodeRef::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::Vault(vault_id) => SubstateId::Vault(*vault_id),
                    _ => panic!("Unexpected"),
                };
                let substate = track.borrow_substate_mut(substate_id);
                substate.convert_to_node();
                substate.vault()
            }
        }
    }

    pub fn system(&self) -> &System {
        match self {
            RENodeRef::Stack(value, id) => id
                .as_ref()
                .map_or(value.root(), |v| value.non_root(v))
                .system(),
            RENodeRef::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::System(component_address) => SubstateId::System(*component_address),
                    _ => panic!("Unexpected"),
                };
                track.borrow_substate(substate_id).raw().system()
            }
        }
    }

    pub fn resource_manager(&self) -> &ResourceManager {
        match self {
            RENodeRef::Stack(value, id) => id
                .as_ref()
                .map_or(value.root(), |v| value.non_root(v))
                .resource_manager(),
            RENodeRef::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::ResourceManager(resource_address) => {
                        SubstateId::ResourceManager(*resource_address)
                    }
                    _ => panic!("Unexpected"),
                };
                track.borrow_substate(substate_id).raw().resource_manager()
            }
        }
    }

    pub fn component_state(&self) -> &ComponentState {
        match self {
            RENodeRef::Stack(value, id) => id
                .as_ref()
                .map_or(value.root(), |v| value.non_root(v))
                .component_state(),
            RENodeRef::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::Component(component_address) => {
                        SubstateId::ComponentState(*component_address)
                    }
                    _ => panic!("Unexpected"),
                };
                track.borrow_substate(substate_id).raw().component_state()
            }
        }
    }

    pub fn component_info(&self) -> &ComponentInfo {
        match self {
            RENodeRef::Stack(value, id) => id
                .as_ref()
                .map_or(value.root(), |v| value.non_root(v))
                .component_info(),
            RENodeRef::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::Component(component_address) => {
                        SubstateId::ComponentInfo(*component_address)
                    }
                    _ => panic!("Unexpected"),
                };
                track.borrow_substate(substate_id).raw().component_info()
            }
        }
    }

    pub fn package(&self) -> &Package {
        match self {
            RENodeRef::Stack(value, id) => id
                .as_ref()
                .map_or(value.root(), |v| value.non_root(v))
                .package(),
            RENodeRef::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::Package(package_address) => SubstateId::Package(*package_address),
                    _ => panic!("Unexpected"),
                };
                track.borrow_substate(substate_id).raw().package()
            }
        }
    }
}

pub enum RENodeRefMut<'f, 's, R: FeeReserve> {
    Stack(&'f mut HeapRootRENode, Option<RENodeId>),
    Track(&'f mut Track<'s, R>, RENodeId),
}

impl<'f, 's, R: FeeReserve> RENodeRefMut<'f, 's, R> {
    pub fn read_scrypto_value(
        &mut self,
        substate_id: &SubstateId,
    ) -> Result<ScryptoValue, RuntimeError> {
        match substate_id {
            SubstateId::ComponentInfo(..) => {
                Ok(ScryptoValue::from_typed(&self.component_info().info()))
            }
            SubstateId::ComponentState(..) => {
                Ok(ScryptoValue::from_slice(self.component_state().state())
                    .expect("Failed to decode component state"))
            }
            SubstateId::NonFungible(.., id) => Ok(self.non_fungible_get(id)),
            SubstateId::KeyValueStoreEntry(.., key) => Ok(self.kv_store_get(key)),
            SubstateId::NonFungibleSpace(..)
            | SubstateId::Global(..)
            | SubstateId::Vault(..)
            | SubstateId::KeyValueStoreSpace(..)
            | SubstateId::Package(..)
            | SubstateId::ResourceManager(..)
            | SubstateId::System(..)
            | SubstateId::Bucket(..)
            | SubstateId::Proof(..)
            | SubstateId::AuthZone(..)
            | SubstateId::Worktop => {
                panic!("Should never have received permissions to read this native type.");
            }
        }
    }

    pub fn replace_value_with_default(&mut self, substate_id: &SubstateId) {
        match substate_id {
            SubstateId::Global(..)
            | SubstateId::ComponentInfo(..)
            | SubstateId::ComponentState(..)
            | SubstateId::NonFungibleSpace(..)
            | SubstateId::KeyValueStoreSpace(..)
            | SubstateId::KeyValueStoreEntry(..)
            | SubstateId::Vault(..)
            | SubstateId::Package(..)
            | SubstateId::ResourceManager(..)
            | SubstateId::System(..)
            | SubstateId::Bucket(..)
            | SubstateId::Proof(..)
            | SubstateId::AuthZone(..)
            | SubstateId::Worktop => {
                panic!("Should not get here");
            }
            SubstateId::NonFungible(.., id) => self.non_fungible_remove(&id),
        }
    }

    pub fn write_value(
        &mut self,
        substate_id: SubstateId,
        value: ScryptoValue,
        child_nodes: HashMap<RENodeId, HeapRootRENode>,
    ) -> Result<(), NodeToSubstateFailure> {
        match substate_id {
            SubstateId::Global(..) => {
                panic!("Should not get here");
            }
            SubstateId::ComponentInfo(..) => {
                panic!("Should not get here");
            }
            SubstateId::ComponentState(..) => {
                self.component_state_set(value, child_nodes)?;
            }
            SubstateId::KeyValueStoreSpace(..) => {
                panic!("Should not get here");
            }
            SubstateId::KeyValueStoreEntry(.., key) => {
                self.kv_store_put(key, value, child_nodes)?;
            }
            SubstateId::NonFungibleSpace(..) => {
                panic!("Should not get here");
            }
            SubstateId::NonFungible(.., id) => self.non_fungible_put(id, value),
            SubstateId::Vault(..) => {
                panic!("Should not get here");
            }
            SubstateId::Package(..) => {
                panic!("Should not get here");
            }
            SubstateId::ResourceManager(..) => {
                panic!("Should not get here");
            }
            SubstateId::System(..) => {
                panic!("Should not get here");
            }
            SubstateId::Bucket(..) => {
                panic!("Should not get here");
            }
            SubstateId::Proof(..) => {
                panic!("Should not get here");
            }
            SubstateId::Worktop => {
                panic!("Should not get here");
            }
            SubstateId::AuthZone(..) => {
                panic!("Should not get here");
            }
        }
        Ok(())
    }

    pub fn kv_store_put(
        &mut self,
        key: Vec<u8>,
        value: ScryptoValue,
        to_store: HashMap<RENodeId, HeapRootRENode>,
    ) -> Result<(), NodeToSubstateFailure> {
        match self {
            RENodeRefMut::Stack(re_value, id) => {
                re_value
                    .get_node_mut(id.as_ref())
                    .kv_store_mut()
                    .put(key, value);
                for (id, val) in to_store {
                    re_value.insert_non_root_nodes(val.to_nodes(id));
                }
            }
            RENodeRefMut::Track(track, node_id) => {
                let parent_substate_id = match node_id {
                    RENodeId::KeyValueStore(kv_store_id) => {
                        SubstateId::KeyValueStoreSpace(*kv_store_id)
                    }
                    _ => panic!("Unexpeceted"),
                };
                track.set_key_value(
                    parent_substate_id,
                    key,
                    Substate::KeyValueStoreEntry(KeyValueStoreEntrySubstate(Some(value.raw))),
                );
                for (id, val) in to_store {
                    insert_non_root_nodes(track, val.to_nodes(id))?;
                }
            }
        }

        Ok(())
    }

    pub fn kv_store_get(&mut self, key: &[u8]) -> ScryptoValue {
        let wrapper = match self {
            RENodeRefMut::Stack(re_value, id) => {
                let store = re_value.get_node_mut(id.as_ref()).kv_store_mut();
                store
                    .get(key)
                    .map(|v| KeyValueStoreEntrySubstate(Some(v.raw)))
                    .unwrap_or(KeyValueStoreEntrySubstate(None))
            }
            RENodeRefMut::Track(track, node_id) => {
                let parent_substate_id = match node_id {
                    RENodeId::KeyValueStore(kv_store_id) => {
                        SubstateId::KeyValueStoreSpace(*kv_store_id)
                    }
                    _ => panic!("Unexpeceted"),
                };
                let substate_value = track.read_key_value(parent_substate_id, key.to_vec());
                substate_value.into()
            }
        };

        // TODO: Cleanup after adding polymorphism support for SBOR
        // For now, we have to use `Vec<u8>` within `KeyValueStoreEntrySubstate`
        // and apply the following ugly conversion.
        let value = wrapper.0.map_or(
            Value::Option {
                value: Box::new(Option::None),
            },
            |v| Value::Option {
                value: Box::new(Some(
                    decode_any(&v).expect("Failed to decode the value in NonFungibleSubstate"),
                )),
            },
        );

        ScryptoValue::from_value(value)
            .expect("Failed to convert non-fungible value to Scrypto value")
    }

    pub fn non_fungible_get(&mut self, id: &NonFungibleId) -> ScryptoValue {
        let wrapper = match self {
            RENodeRefMut::Stack(value, re_id) => {
                let non_fungible_set = re_id
                    .as_ref()
                    .map_or(value.root(), |v| value.non_root(v))
                    .non_fungibles();
                non_fungible_set
                    .get(id)
                    .cloned()
                    .map(|v| NonFungibleSubstate(Some(v)))
                    .unwrap_or(NonFungibleSubstate(None))
            }
            RENodeRefMut::Track(track, node_id) => {
                let parent_substate_id = match node_id {
                    RENodeId::ResourceManager(resource_address) => {
                        SubstateId::NonFungibleSpace(*resource_address)
                    }
                    _ => panic!("Unexpeceted"),
                };
                let substate_value = track.read_key_value(parent_substate_id, id.to_vec());
                substate_value.into()
            }
        };

        ScryptoValue::from_typed(&wrapper)
    }

    pub fn non_fungible_remove(&mut self, id: &NonFungibleId) {
        match self {
            RENodeRefMut::Stack(..) => {
                panic!("Not supported");
            }
            RENodeRefMut::Track(track, node_id) => {
                let parent_substate_id = match node_id {
                    RENodeId::ResourceManager(resource_address) => {
                        SubstateId::NonFungibleSpace(*resource_address)
                    }
                    _ => panic!("Unexpeceted"),
                };
                track.set_key_value(
                    parent_substate_id,
                    id.to_vec(),
                    Substate::NonFungible(NonFungibleSubstate(None)),
                );
            }
        }
    }

    pub fn non_fungible_put(&mut self, id: NonFungibleId, value: ScryptoValue) {
        match self {
            RENodeRefMut::Stack(re_value, re_id) => {
                let wrapper: NonFungibleSubstate = scrypto_decode(&value.raw)
                    .expect("Attempted to put non-NonFungibleSubstate for non-fungible.");

                let non_fungible_set = re_value.get_node_mut(re_id.as_ref()).non_fungibles_mut();
                if let Some(non_fungible) = wrapper.0 {
                    non_fungible_set.insert(id, non_fungible);
                } else {
                    panic!("TODO: invalidate this code path and possibly consolidate `non_fungible_remove` and `non_fungible_put`")
                }
            }
            RENodeRefMut::Track(track, node_id) => {
                let parent_substate_id = match node_id {
                    RENodeId::ResourceManager(resource_address) => {
                        SubstateId::NonFungibleSpace(*resource_address)
                    }
                    _ => panic!("Unexpeceted"),
                };
                let wrapper: NonFungibleSubstate = scrypto_decode(&value.raw)
                    .expect("Attempted to put non-NonFungibleSubstate for non-fungible.");
                track.set_key_value(
                    parent_substate_id,
                    id.to_vec(),
                    Substate::NonFungible(wrapper),
                );
            }
        }
    }

    pub fn component_state_set(
        &mut self,
        value: ScryptoValue,
        to_store: HashMap<RENodeId, HeapRootRENode>,
    ) -> Result<(), NodeToSubstateFailure> {
        match self {
            RENodeRefMut::Stack(re_value, id) => {
                let component_state = re_value.get_node_mut(id.as_ref()).component_state_mut();
                component_state.set_state(value.raw);
                for (id, val) in to_store {
                    re_value.insert_non_root_nodes(val.to_nodes(id));
                }
            }
            RENodeRefMut::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::Component(component_address) => {
                        SubstateId::ComponentState(*component_address)
                    }
                    _ => panic!("Unexpeceted"),
                };
                *track.borrow_substate_mut(substate_id) =
                    SubstateCache::Raw(ComponentState::new(value.raw).into());
                for (id, val) in to_store {
                    insert_non_root_nodes(track, val.to_nodes(id))?;
                }
            }
        }
        Ok(())
    }

    pub fn component_info(&mut self) -> &ComponentInfo {
        match self {
            RENodeRefMut::Stack(re_value, id) => {
                re_value.get_node_mut(id.as_ref()).component_info()
            }
            RENodeRefMut::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::Component(component_address) => {
                        SubstateId::ComponentInfo(*component_address)
                    }
                    _ => panic!("Unexpeceted"),
                };
                track.borrow_substate(substate_id).raw().component_info()
            }
        }
    }

    pub fn component_state(&mut self) -> &ComponentState {
        match self {
            RENodeRefMut::Stack(re_value, id) => {
                re_value.get_node_mut(id.as_ref()).component_state()
            }
            RENodeRefMut::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::Component(component_address) => {
                        SubstateId::ComponentState(*component_address)
                    }
                    _ => panic!("Unexpeceted"),
                };
                track.borrow_substate(substate_id).raw().component_state()
            }
        }
    }

    pub fn vault_mut(&mut self) -> &mut Vault {
        match self {
            RENodeRefMut::Stack(re_value, id) => re_value.get_node_mut(id.as_ref()).vault_mut(),
            RENodeRefMut::Track(track, node_id) => {
                let substate_id = match node_id {
                    RENodeId::Vault(vault_id) => SubstateId::Vault(*vault_id),
                    _ => panic!("Unexpeceted"),
                };
                let substate = track.borrow_substate_mut(substate_id);
                substate.convert_to_node();
                substate.vault_mut()
            }
        }
    }
}

pub fn verify_stored_value_update(
    old: &HashSet<RENodeId>,
    missing: &HashSet<RENodeId>,
) -> Result<(), RuntimeError> {
    // TODO: optimize intersection search
    for old_id in old.iter() {
        if !missing.contains(&old_id) {
            return Err(RuntimeError::KernelError(KernelError::StoredNodeRemoved(
                old_id.clone(),
            )));
        }
    }

    for missing_id in missing.iter() {
        if !old.contains(missing_id) {
            return Err(RuntimeError::KernelError(KernelError::RENodeNotFound(
                *missing_id,
            )));
        }
    }

    Ok(())
}

pub fn insert_non_root_nodes<'s, R: FeeReserve>(
    track: &mut Track<'s, R>,
    values: HashMap<RENodeId, HeapRENode>,
) -> Result<(), NodeToSubstateFailure> {
    for (id, node) in values {
        match node {
            HeapRENode::Vault(vault) => {
                let resource = vault
                    .resource()
                    .map_err(|_| NodeToSubstateFailure::VaultPartiallyLocked)?;
                track.create_uuid_substate(SubstateId::Vault(id.into()), VaultSubstate(resource));
            }
            HeapRENode::Component(component, component_state) => {
                let component_address = id.into();
                track.create_uuid_substate(SubstateId::ComponentInfo(component_address), component);
                track.create_uuid_substate(
                    SubstateId::ComponentState(component_address),
                    component_state,
                );
            }
            HeapRENode::KeyValueStore(store) => {
                let id = id.into();
                let substate_id = SubstateId::KeyValueStoreSpace(id);
                for (k, v) in store.store {
                    track.set_key_value(
                        substate_id.clone(),
                        k,
                        KeyValueStoreEntrySubstate(Some(v.raw)),
                    );
                }
            }
            _ => panic!("Invalid node being persisted: {:?}", node),
        }
    }
    Ok(())
}
