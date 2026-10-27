//! Read-only iteration over one component type.

use crate::Entity;

/// Iterates `(Entity, &T)` for every entity with a `T` component. See
/// [`World::query`](crate::World::query).
pub struct Query<'w, T> {
    pub(crate) inner: Box<dyn Iterator<Item = (Entity, &'w T)> + 'w>,
}

impl<T> std::fmt::Debug for Query<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query").finish_non_exhaustive()
    }
}

impl<'w, T> Iterator for Query<'w, T> {
    type Item = (Entity, &'w T);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
