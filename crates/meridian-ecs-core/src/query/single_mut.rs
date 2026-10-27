//! Mutable iteration over one component type.

use crate::Entity;

/// Iterates `(Entity, &mut T)` for every entity with a `T` component. See
/// [`World::query_mut`](crate::World::query_mut).
pub struct QueryMut<'w, T> {
    pub(crate) inner: Box<dyn Iterator<Item = (Entity, &'w mut T)> + 'w>,
}

impl<T> std::fmt::Debug for QueryMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryMut").finish_non_exhaustive()
    }
}

impl<'w, T> Iterator for QueryMut<'w, T> {
    type Item = (Entity, &'w mut T);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
