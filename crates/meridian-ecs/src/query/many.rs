//! Read-only iteration over several component types at once.

use std::slice;

use super::ReadSelection;
use super::selection::sealed::SelectionRows;
use crate::{Archetype, Entity, Tick};

/// Iterates `(Entity, (&A, &B, ..))` for every entity that has all the
/// listed components. See [`World::query_many`](crate::World::query_many).
pub struct QueryMany<'w, Q: ReadSelection> {
    archetypes: slice::Iter<'w, Archetype>,
    since: Tick,
    rows: Option<<Q as SelectionRows>::Rows<'w>>,
}

impl<'w, Q: ReadSelection> QueryMany<'w, Q> {
    pub(crate) fn new(archetypes: slice::Iter<'w, Archetype>, since: Tick) -> Self {
        Self {
            archetypes,
            since,
            rows: None,
        }
    }
}

impl<Q: ReadSelection> std::fmt::Debug for QueryMany<'_, Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryMany").finish_non_exhaustive()
    }
}

impl<'w, Q: ReadSelection> Iterator for QueryMany<'w, Q> {
    type Item = (Entity, <Q as SelectionRows>::RowItem<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.rows.as_mut().and_then(Iterator::next) {
                return Some(item);
            }
            self.rows = Q::rows(self.archetypes.next()?, self.since);
        }
    }
}
