//! Why a world could not be snapshotted.

use std::any::TypeId;

/// A failed [`World::snapshot`](crate::World::snapshot).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotError {
    /// Entities hold component types the [`Cloners`](crate::Cloners) do not
    /// know. Sorted, so the same world always reports the same list.
    #[error("{} component type(s) cannot be cloned; register them in `Cloners`", .0.len())]
    Unregistered(Vec<TypeId>),
}
