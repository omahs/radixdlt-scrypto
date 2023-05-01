use radix_engine::system::node_modules::type_info::TypeInfoSubstate;
use radix_engine::track::db_key_mapper::{MappedSubstateDatabase, SpreadPrefixKeyMapper};
use radix_engine_interface::blueprints::resource::{
    LiquidNonFungibleVault, FUNGIBLE_VAULT_BLUEPRINT, NON_FUNGIBLE_VAULT_BLUEPRINT,
};
use radix_engine_interface::constants::RESOURCE_MANAGER_PACKAGE;
use radix_engine_interface::data::scrypto::model::NonFungibleLocalId;
use radix_engine_interface::data::scrypto::scrypto_decode;
use radix_engine_interface::types::{
    FungibleVaultOffset, IndexedScryptoValue, ModuleNumber, NonFungibleVaultOffset, ObjectInfo,
    ResourceAddress, TypeInfoOffset, ACCESS_RULES_BASE_MODULE, METADATA_BASE_MODULE,
    OBJECT_BASE_MODULE, ROYALTY_BASE_MODULE, TYPE_INFO_BASE_MODULE,
};
use radix_engine_interface::{blueprints::resource::LiquidFungibleResource, types::NodeId};
use radix_engine_store_interface::interface::{DbSortKey, SubstateDatabase};
use sbor::rust::prelude::*;

pub struct StateTreeTraverser<'s, 'v, S: SubstateDatabase, V: StateTreeVisitor> {
    substate_db: &'s S,
    visitor: &'v mut V,
    max_depth: u32,
}

pub trait StateTreeVisitor {
    fn visit_fungible_vault(
        &mut self,
        _vault_id: NodeId,
        _address: &ResourceAddress,
        _resource: &LiquidFungibleResource,
    ) {
    }

    fn visit_non_fungible_vault(
        &mut self,
        _vault_id: NodeId,
        _address: &ResourceAddress,
        _resource: &LiquidNonFungibleVault,
    ) {
    }

    fn visit_non_fungible(
        &mut self,
        _vault_id: NodeId,
        _address: &ResourceAddress,
        _id: &NonFungibleLocalId,
    ) {
    }

    fn visit_node_id(
        &mut self,
        _parent_id: Option<&(NodeId, ModuleNumber, DbSortKey)>,
        _node_id: &NodeId,
        _depth: u32,
    ) {
    }
}

impl<'s, 'v, S: SubstateDatabase, V: StateTreeVisitor> StateTreeTraverser<'s, 'v, S, V> {
    pub fn new(substate_db: &'s S, visitor: &'v mut V, max_depth: u32) -> Self {
        StateTreeTraverser {
            substate_db,
            visitor,
            max_depth,
        }
    }

    pub fn traverse_all_descendents(
        &mut self,
        parent_node_id: Option<&(NodeId, ModuleNumber, DbSortKey)>,
        node_id: NodeId,
    ) {
        self.traverse_recursive(parent_node_id, node_id, 0)
    }

    fn traverse_recursive(
        &mut self,
        parent: Option<&(NodeId, ModuleNumber, DbSortKey)>,
        node_id: NodeId,
        depth: u32,
    ) {
        if depth > self.max_depth {
            return;
        }

        // Notify visitor
        self.visitor.visit_node_id(parent, &node_id, depth);

        // Load type info
        let type_info = self
            .substate_db
            .get_mapped_substate::<SpreadPrefixKeyMapper, TypeInfoSubstate>(
                &node_id,
                TYPE_INFO_BASE_MODULE,
                &TypeInfoOffset::TypeInfo.into(),
            )
            .expect("Missing TypeInfo substate");

        match type_info {
            TypeInfoSubstate::KeyValueStore(_) => {
                for (db_sort_key, value) in self
                    .substate_db
                    .list_mapped_substates::<SpreadPrefixKeyMapper>(&node_id, OBJECT_BASE_MODULE)
                {
                    let (_, owned_nodes, _) = IndexedScryptoValue::from_vec(value)
                        .expect("Substate is not a scrypto value")
                        .unpack();
                    for child_node_id in owned_nodes {
                        self.traverse_recursive(
                            Some(&(node_id, OBJECT_BASE_MODULE, db_sort_key.clone())),
                            child_node_id,
                            depth + 1,
                        );
                    }
                }
            }
            TypeInfoSubstate::Index | TypeInfoSubstate::SortedIndex => {}
            TypeInfoSubstate::Object(ObjectInfo {
                blueprint,
                outer_object,
                global: _,
                instance_schema: _,
            }) => {
                if blueprint.package_address.eq(&RESOURCE_MANAGER_PACKAGE)
                    && blueprint.blueprint_name.eq(FUNGIBLE_VAULT_BLUEPRINT)
                {
                    let liquid = self
                        .substate_db
                        .get_mapped_substate::<SpreadPrefixKeyMapper, LiquidFungibleResource>(
                            &node_id,
                            OBJECT_BASE_MODULE,
                            &FungibleVaultOffset::LiquidFungible.into(),
                        )
                        .expect("Broken database");

                    self.visitor.visit_fungible_vault(
                        node_id,
                        &ResourceAddress::new_or_panic(outer_object.unwrap().into()),
                        &liquid,
                    );
                } else if blueprint.package_address.eq(&RESOURCE_MANAGER_PACKAGE)
                    && blueprint.blueprint_name.eq(NON_FUNGIBLE_VAULT_BLUEPRINT)
                {
                    let liquid = self
                        .substate_db
                        .get_mapped_substate::<SpreadPrefixKeyMapper, LiquidNonFungibleVault>(
                            &node_id,
                            OBJECT_BASE_MODULE,
                            &NonFungibleVaultOffset::LiquidNonFungible.into(),
                        )
                        .expect("Broken database");

                    self.visitor.visit_non_fungible_vault(
                        node_id,
                        &ResourceAddress::new_or_panic(outer_object.unwrap().into()),
                        &liquid,
                    );

                    let ids = self
                        .substate_db
                        .list_mapped_substates::<SpreadPrefixKeyMapper>(
                            liquid.ids.as_node_id(),
                            OBJECT_BASE_MODULE,
                        );
                    for (_key, value) in ids {
                        let non_fungible_local_id: NonFungibleLocalId =
                            scrypto_decode(&value).unwrap();

                        self.visitor.visit_non_fungible(
                            node_id,
                            &ResourceAddress::new_or_panic(outer_object.unwrap().into()),
                            &non_fungible_local_id,
                        );
                    }
                } else {
                    for t in [
                        TYPE_INFO_BASE_MODULE,
                        METADATA_BASE_MODULE,
                        ROYALTY_BASE_MODULE,
                        ACCESS_RULES_BASE_MODULE,
                        OBJECT_BASE_MODULE,
                    ] {
                        // List all iterable modules (currently `ObjectState` & `Metadata`)
                        let x = self
                            .substate_db
                            .list_mapped_substates::<SpreadPrefixKeyMapper>(&node_id, t);
                        for (db_sort_key, substate_value) in x {
                            let (_, owned_nodes, _) = IndexedScryptoValue::from_vec(substate_value)
                                .expect("Substate is not a scrypto value")
                                .unpack();
                            for child_node_id in owned_nodes {
                                self.traverse_recursive(
                                    Some(&(node_id, OBJECT_BASE_MODULE, db_sort_key.clone())),
                                    child_node_id,
                                    depth + 1,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
