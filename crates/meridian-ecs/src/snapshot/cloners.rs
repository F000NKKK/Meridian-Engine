//! How to copy each component type's storage, for world snapshots.

use std::any::TypeId;
use std::collections::HashMap;

use crate::{AnyColumn, Column, Component};

type CloneColumn = fn(&dyn AnyColumn) -> Option<Box<dyn AnyColumn>>;

/// The component types a [`World`](crate::World) snapshot can copy. A world is
/// type-erased, so it cannot clone a column of a type it was never told about:
/// the application, which knows its components, registers the `Clone` ones
/// here (`meridian-data-substrate`'s registry already requires `Clone`).
#[derive(Debug, Default)]
pub struct Cloners {
    by_type: HashMap<TypeId, CloneColumn>,
}

impl Cloners {
    /// A set that can clone nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `T`. Registering a type twice is harmless.
    #[must_use]
    pub fn with<T: Component + Clone>(mut self) -> Self {
        self.by_type.insert(TypeId::of::<T>(), clone_column::<T>);
        self
    }

    /// Whether `type_id` can be cloned.
    #[must_use]
    pub fn contains(&self, type_id: TypeId) -> bool {
        self.by_type.contains_key(&type_id)
    }

    /// A copy of `column`, or `None` if its type was not registered.
    pub(crate) fn clone_column(
        &self,
        type_id: TypeId,
        column: &dyn AnyColumn,
    ) -> Option<Box<dyn AnyColumn>> {
        self.by_type.get(&type_id)?(column)
    }
}

fn clone_column<T: Component + Clone>(column: &dyn AnyColumn) -> Option<Box<dyn AnyColumn>> {
    let column = column.as_any().downcast_ref::<Column<T>>()?;
    Some(Box::new(Column::<T>(column.0.clone(), column.1)))
}
