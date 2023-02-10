use crate::rust::borrow::Cow;
use crate::rust::vec::Vec;
use crate::*;

/// An array of custom type kinds, and associated extra information which can attach to the type kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema<E: CustomTypeExtension> {
    pub type_kinds: Vec<SchemaTypeKind<E>>,
    pub type_metadata: Vec<NovelTypeMetadata>,
    pub type_validations: Vec<SchemaTypeValidation<E>>,
}

pub type SchemaTypeKind<E> =
    TypeKind<<E as CustomTypeExtension>::CustomValueKind, SchemaCustomTypeKind<E>, LocalTypeIndex>;
pub type SchemaCustomTypeKind<E> = <E as CustomTypeExtension>::CustomTypeKind<LocalTypeIndex>;
pub type SchemaTypeValidation<E> = TypeValidation<<E as CustomTypeExtension>::CustomTypeValidation>;
pub type SchemaCustomTypeValidation<E> = <E as CustomTypeExtension>::CustomTypeValidation;

// TODO: Could get rid of the Cow by using some per-custom type once_cell to cache basic well-known-types,
//       and return references to the static cached values
pub struct ResolvedTypeData<'a, E: CustomTypeExtension> {
    pub kind:
        Cow<'a, TypeKind<E::CustomValueKind, E::CustomTypeKind<LocalTypeIndex>, LocalTypeIndex>>,
    pub metadata: Cow<'a, TypeMetadata>,
}

impl<E: CustomTypeExtension> Schema<E> {
    pub fn resolve<'a>(&'a self, type_ref: LocalTypeIndex) -> Option<ResolvedTypeData<'a, E>> {
        match type_ref {
            LocalTypeIndex::WellKnown(index) => {
                resolve_well_known_type::<E>(index).map(|local_type_data| ResolvedTypeData {
                    kind: Cow::Owned(local_type_data.kind),
                    metadata: Cow::Owned(local_type_data.metadata),
                })
            }
            LocalTypeIndex::SchemaLocalIndex(index) => {
                match (self.type_kinds.get(index), self.type_metadata.get(index)) {
                    (Some(schema), Some(novel_metadata)) => Some(ResolvedTypeData {
                        kind: Cow::Borrowed(schema),
                        metadata: Cow::Borrowed(&novel_metadata.type_metadata),
                    }),
                    (None, None) => None,
                    _ => panic!("Index existed in exactly one of schema and naming"),
                }
            }
        }
    }

    pub fn validate(&self) -> Result<(), SchemaValidationError> {
        validate_schema(self)
    }
}
