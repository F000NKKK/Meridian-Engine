//! A tuple of [`Fetch`](crate::Fetch) elements that a query walks together.

use std::any::TypeId;

use crate::{Archetype, Entity, Tick};

/// A tuple of `&T` / `&mut T` (two to four elements) naming the components a
/// query reads and writes together. Implemented for those tuples only.
pub trait Selection: sealed::SelectionRows {}

/// A [`Selection`] with no `&mut` element, so it can run on a shared `&World`.
pub trait ReadSelection: Selection + sealed::ReadSelectionRows {}

/// The archetype-facing half of the query traits. Sealed: it names crate
/// internals, and nothing outside this crate may implement a query tuple.
pub(crate) mod sealed {
    use super::{Archetype, Entity, Tick, TypeId};

    pub trait SelectionRows {
        /// The rows of one archetype, front to back.
        type Rows<'w>: Iterator<Item = (Entity, Self::RowItem<'w>)>;
        /// The tuple of fetched items for one row.
        type RowItem<'w>;

        /// The component of every element, in tuple order.
        fn component_ids() -> Vec<TypeId>;

        /// Borrows the columns of `archetype`, or `None` when it does not
        /// match: a missing column, a filter that refuses it, or no change
        /// after `since`. Written columns are stamped changed at `now`. The
        /// caller has checked that `component_ids` are pairwise distinct.
        fn rows_mut(archetype: &mut Archetype, since: Tick, now: Tick) -> Option<Self::Rows<'_>>;
    }

    pub trait ReadSelectionRows: SelectionRows {
        /// Same as [`SelectionRows::rows_mut`] through a shared borrow.
        fn rows(archetype: &Archetype, since: Tick) -> Option<Self::Rows<'_>>;
    }
}
