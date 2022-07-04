use sbor::rust::collections::*;
use sbor::rust::vec::Vec;
use scrypto::engine::types::*;
use scrypto::values::ScryptoValue;

use crate::engine::*;
use crate::model::*;

#[derive(Debug)]
pub enum RENode {
    Bucket(Bucket),
    Proof(Proof),
    Vault(Vault),
    KeyValueStore(PreCommittedKeyValueStore),
    Component(Component),
    Package(ValidatedPackage),
    Resource(ResourceManager),
    NonFungibles(HashMap<NonFungibleId, NonFungible>),
}

impl RENode {
    pub fn resource_manager(&self) -> &ResourceManager {
        match self {
            RENode::Resource(resource_manager) => resource_manager,
            _ => panic!("Expected to be a resource manager"),
        }
    }

    pub fn resource_manager_mut(&mut self) -> &mut ResourceManager {
        match self {
            RENode::Resource(resource_manager) => resource_manager,
            _ => panic!("Expected to be a resource manager"),
        }
    }

    pub fn non_fungibles(&self) -> &HashMap<NonFungibleId, NonFungible> {
        match self {
            RENode::NonFungibles(non_fungibles) => non_fungibles,
            _ => panic!("Expected to be non fungibles"),
        }
    }

    pub fn non_fungibles_mut(&mut self) -> &mut HashMap<NonFungibleId, NonFungible> {
        match self {
            RENode::NonFungibles(non_fungibles) => non_fungibles,
            _ => panic!("Expected to be non fungibles"),
        }
    }

    pub fn package(&self) -> &ValidatedPackage {
        match self {
            RENode::Package(package) => package,
            _ => panic!("Expected to be a package"),
        }
    }

    pub fn component(&self) -> &Component {
        match self {
            RENode::Component(component) => component,
            _ => panic!("Expected to be a store"),
        }
    }

    pub fn component_mut(&mut self) -> &mut Component {
        match self {
            RENode::Component(component) => component,
            _ => panic!("Expected to be a store"),
        }
    }

    pub fn kv_store(&self) -> &PreCommittedKeyValueStore {
        match self {
            RENode::KeyValueStore(store) => store,
            _ => panic!("Expected to be a store"),
        }
    }

    pub fn kv_store_mut(&mut self) -> &mut PreCommittedKeyValueStore {
        match self {
            RENode::KeyValueStore(store) => store,
            _ => panic!("Expected to be a store"),
        }
    }

    pub fn vault(&self) -> &Vault {
        match self {
            RENode::Vault(vault) => vault,
            _ => panic!("Expected to be a vault"),
        }
    }

    pub fn vault_mut(&mut self) -> &mut Vault {
        match self {
            RENode::Vault(vault) => vault,
            _ => panic!("Expected to be a vault"),
        }
    }

    pub fn verify_can_move(&self) -> Result<(), RuntimeError> {
        match self {
            RENode::Bucket(bucket) => {
                if bucket.is_locked() {
                    Err(RuntimeError::CantMoveLockedBucket)
                } else {
                    Ok(())
                }
            }
            RENode::Proof(proof) => {
                if proof.is_restricted() {
                    Err(RuntimeError::CantMoveRestrictedProof)
                } else {
                    Ok(())
                }
            }
            RENode::KeyValueStore(..) => Ok(()),
            RENode::Component(..) => Ok(()),
            RENode::Vault(..) => Ok(()),
            RENode::Resource(..) => Ok(()),
            RENode::NonFungibles(..) => Ok(()),
            RENode::Package(..) => Ok(()),
        }
    }

    pub fn verify_can_persist(&self) -> Result<(), RuntimeError> {
        match self {
            RENode::KeyValueStore { .. } => Ok(()),
            RENode::Component { .. } => Ok(()),
            RENode::Vault(..) => Ok(()),
            RENode::Resource(..) => Err(RuntimeError::ValueNotAllowed),
            RENode::NonFungibles(..) => Err(RuntimeError::ValueNotAllowed),
            RENode::Package(..) => Err(RuntimeError::ValueNotAllowed),
            RENode::Bucket(..) => Err(RuntimeError::ValueNotAllowed),
            RENode::Proof(..) => Err(RuntimeError::ValueNotAllowed),
        }
    }

    pub fn try_drop(self) -> Result<(), DropFailure> {
        match self {
            RENode::Package(..) => Err(DropFailure::Package),
            RENode::Vault(..) => Err(DropFailure::Vault),
            RENode::KeyValueStore(..) => Err(DropFailure::KeyValueStore),
            RENode::Component(..) => Err(DropFailure::Component),
            RENode::Bucket(..) => Err(DropFailure::Bucket),
            RENode::Resource(..) => Err(DropFailure::Resource),
            RENode::NonFungibles(..) => Err(DropFailure::Resource),
            RENode::Proof(proof) => {
                proof.drop();
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub struct REValue {
    pub root: RENode,
    pub non_root_nodes: HashMap<ValueId, RENode>,
}

impl REValue {
    pub fn root(&self) -> &RENode {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut RENode {
        &mut self.root
    }

    pub fn non_root(&self, id: &ValueId) -> &RENode {
        self.non_root_nodes.get(id).unwrap()
    }

    pub fn non_root_mut(&mut self, id: &ValueId) -> &mut RENode {
        self.non_root_nodes.get_mut(id).unwrap()
    }

    pub fn get(&self, id: Option<&ValueId>) -> &RENode {
        if let Some(value_id) = id {
            self.non_root_nodes.get(value_id).unwrap()
        } else {
            &self.root
        }
    }

    pub fn get_mut(&mut self, id: Option<&ValueId>) -> &mut RENode {
        if let Some(value_id) = id {
            self.non_root_nodes.get_mut(value_id).unwrap()
        } else {
            &mut self.root
        }
    }

    pub fn insert_non_root_nodes(&mut self, values: HashMap<ValueId, RENode>) {
        for (id, value) in values {
            self.non_root_nodes.insert(id, value);
        }
    }

    pub fn to_nodes(self, root_id: ValueId) -> HashMap<ValueId, RENode> {
        let mut nodes = self.non_root_nodes;
        nodes.insert(root_id, self.root);
        nodes
    }

    pub fn try_drop(self) -> Result<(), DropFailure> {
        self.root.try_drop()
    }

    pub fn all_descendants(&self) -> Vec<ValueId> {
        let mut descendents = Vec::new();
        for (id, ..) in self.non_root_nodes.iter() {
            descendents.push(*id);
        }
        descendents
    }

    /*
    pub fn get_child(&self, ancestors: &[KeyValueStoreId], id: &ValueId) -> &REValue {
        if ancestors.is_empty() {
            let value = self
                .non_root_nodes
                .get(id)
                .expect("Value expected to exist");
            return value;
        }

        let (first, rest) = ancestors.split_first().unwrap();
        let value = self
            .non_root_nodes
            .get(&ValueId::KeyValueStore(*first))
            .unwrap();
        value.get_child(rest, id)
    }

    pub fn get_child_mut(
        &mut self,
        ancestors: &[KeyValueStoreId],
        id: &ValueId,
    ) -> &mut REValue {
        if ancestors.is_empty() {
            let value = self
                .non_root_nodes
                .get_mut(id)
                .expect("Value expected to exist");
            return value;
        }

        let (first, rest) = ancestors.split_first().unwrap();
        let value = self
            .non_root_nodes
            .get_mut(&ValueId::KeyValueStore(*first))
            .unwrap();
        value.get_child_mut(rest, id)
    }
     */
}

impl Into<Bucket> for REValue {
    fn into(self) -> Bucket {
        match self.root {
            RENode::Bucket(bucket) => bucket,
            _ => panic!("Expected to be a bucket"),
        }
    }
}

impl Into<Proof> for REValue {
    fn into(self) -> Proof {
        match self.root {
            RENode::Proof(proof) => proof,
            _ => panic!("Expected to be a proof"),
        }
    }
}

impl Into<HashMap<NonFungibleId, NonFungible>> for REValue {
    fn into(self) -> HashMap<NonFungibleId, NonFungible> {
        match self.root {
            RENode::NonFungibles(non_fungibles) => non_fungibles,
            _ => panic!("Expected to be non fungibles"),
        }
    }
}

#[derive(Debug)]
pub enum REComplexValue {
    Component(Component),
}

impl REComplexValue {
    pub fn get_children(&self) -> Result<HashSet<ValueId>, RuntimeError> {
        match self {
            REComplexValue::Component(component) => {
                let value = ScryptoValue::from_slice(component.state())
                    .map_err(RuntimeError::DecodeError)?;
                Ok(value.value_ids())
            }
        }
    }

    pub fn into_re_value(self, non_root_values: HashMap<ValueId, REValue>) -> REValue {
        let mut non_root_nodes = HashMap::new();
        for (id, val) in non_root_values {
            non_root_nodes.extend(val.to_nodes(id));
        }
        match self {
            REComplexValue::Component(component) => REValue {
                root: RENode::Component(component),
                non_root_nodes,
            },
        }
    }
}

#[derive(Debug)]
pub enum REPrimitiveValue {
    Package(ValidatedPackage),
    Bucket(Bucket),
    Proof(Proof),
    KeyValue(PreCommittedKeyValueStore),
    Resource(ResourceManager),
    NonFungibles(ResourceAddress, HashMap<NonFungibleId, NonFungible>),
    Vault(Vault),
}

#[derive(Debug)]
pub enum REValueByComplexity {
    Primitive(REPrimitiveValue),
    Complex(REComplexValue),
}

impl Into<REValue> for REPrimitiveValue {
    fn into(self) -> REValue {
        let root = match self {
            REPrimitiveValue::Resource(resource_manager) => RENode::Resource(resource_manager),
            REPrimitiveValue::NonFungibles(_resource_address, non_fungibles) => {
                RENode::NonFungibles(non_fungibles)
            }
            REPrimitiveValue::Package(package) => RENode::Package(package),
            REPrimitiveValue::Bucket(bucket) => RENode::Bucket(bucket),
            REPrimitiveValue::Proof(proof) => RENode::Proof(proof),
            REPrimitiveValue::KeyValue(store) => RENode::KeyValueStore(store),
            REPrimitiveValue::Vault(vault) => RENode::Vault(vault),
        };
        REValue {
            root,
            non_root_nodes: HashMap::new(),
        }
    }
}

impl Into<REValueByComplexity> for ResourceManager {
    fn into(self) -> REValueByComplexity {
        REValueByComplexity::Primitive(REPrimitiveValue::Resource(self))
    }
}

impl Into<REValueByComplexity> for (ResourceAddress, HashMap<NonFungibleId, NonFungible>) {
    fn into(self) -> REValueByComplexity {
        REValueByComplexity::Primitive(REPrimitiveValue::NonFungibles(self.0, self.1))
    }
}

impl Into<REValueByComplexity> for Bucket {
    fn into(self) -> REValueByComplexity {
        REValueByComplexity::Primitive(REPrimitiveValue::Bucket(self))
    }
}

impl Into<REValueByComplexity> for Proof {
    fn into(self) -> REValueByComplexity {
        REValueByComplexity::Primitive(REPrimitiveValue::Proof(self))
    }
}

impl Into<REValueByComplexity> for Vault {
    fn into(self) -> REValueByComplexity {
        REValueByComplexity::Primitive(REPrimitiveValue::Vault(self))
    }
}

impl Into<REValueByComplexity> for PreCommittedKeyValueStore {
    fn into(self) -> REValueByComplexity {
        REValueByComplexity::Primitive(REPrimitiveValue::KeyValue(self))
    }
}

impl Into<REValueByComplexity> for ValidatedPackage {
    fn into(self) -> REValueByComplexity {
        REValueByComplexity::Primitive(REPrimitiveValue::Package(self))
    }
}

impl Into<REValueByComplexity> for Component {
    fn into(self) -> REValueByComplexity {
        REValueByComplexity::Complex(REComplexValue::Component(self))
    }
}
