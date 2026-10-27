//! An archetype: the entities that share one component set, stored column-wise.

use crate::AnyColumn;
use crate::Column;
use crate::Component;
use crate::Entity;
use std::any::TypeId;
use std::collections::HashMap;

/// The set of component types an entity has; entities sharing an archetype
/// share Structure-of-Arrays storage (each `Column<T>` here is one `SoA`
/// column, keyed by `TypeId`).
pub(crate) struct Archetype {
    pub(crate) type_ids: Vec<TypeId>, // sorted; the archetype's identity
    pub(crate) entities: Vec<Entity>, // row -> Entity
    pub(crate) columns: HashMap<TypeId, Box<dyn AnyColumn>>,
}

impl Archetype {
    /// The `T` column of an archetype whose signature contains `T`.
    pub(crate) fn column_mut<T: Component>(&mut self) -> &mut Vec<T> {
        &mut self
            .columns
            .get_mut(&TypeId::of::<T>())
            .and_then(|column| column.as_any_mut().downcast_mut::<Column<T>>())
            .expect("an archetype has a `Column<T>` for every `T` in its signature")
            .0
    }
}

impl std::fmt::Debug for Archetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Archetype")
            .field("type_ids", &self.type_ids)
            .field("entities", &self.entities)
            .field("column_count", &self.columns.len())
            .finish()
    }
}
