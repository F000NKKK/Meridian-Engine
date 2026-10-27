//! Mutable iteration over several component types at once.

use std::collections::HashSet;
use std::slice;

use super::Selection;
use super::selection::sealed::SelectionRows;
use crate::{Archetype, Entity, Tick};

/// Iterates `(Entity, (&A, &mut B, ..))` for every entity that has all the
/// listed components. See [`World::query_many_mut`](crate::World::query_many_mut).
pub struct QueryManyMut<'w, Q: Selection> {
    archetypes: slice::IterMut<'w, Archetype>,
    since: Tick,
    now: Tick,
    rows: Option<<Q as SelectionRows>::Rows<'w>>,
}

impl<'w, Q: Selection> QueryManyMut<'w, Q> {
    /// # Panics
    ///
    /// If `Q` names one component type twice: `(&mut T, &T)` would alias.
    pub(crate) fn new(archetypes: slice::IterMut<'w, Archetype>, since: Tick, now: Tick) -> Self {
        let ids = Q::component_ids();
        let distinct = ids.iter().collect::<HashSet<_>>().len();
        assert_eq!(
            distinct,
            ids.len(),
            "query_many_mut: a component type appears twice in the query tuple"
        );
        Self {
            archetypes,
            since,
            now,
            rows: None,
        }
    }
}

impl<Q: Selection> std::fmt::Debug for QueryManyMut<'_, Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryManyMut").finish_non_exhaustive()
    }
}

impl<'w, Q: Selection> Iterator for QueryManyMut<'w, Q> {
    type Item = (Entity, <Q as SelectionRows>::RowItem<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.rows.as_mut().and_then(Iterator::next) {
                return Some(item);
            }
            self.rows = Q::rows_mut(self.archetypes.next()?, self.since, self.now);
        }
    }
}
