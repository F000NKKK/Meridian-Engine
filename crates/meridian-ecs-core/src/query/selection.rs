//! A tuple of [`Fetch`](crate::Fetch) elements that a query walks together.

use std::any::TypeId;

use crate::Archetype;
use crate::Entity;

/// A tuple of `&T` / `&mut T` (two to four elements) naming the components a
/// query reads and writes together. Implemented for those tuples only.
pub trait Selection: sealed::SelectionRows {}

/// A [`Selection`] with no `&mut` element, so it can run on a shared `&World`.
pub trait ReadSelection: Selection + sealed::ReadSelectionRows {}

/// The archetype-facing half of the query traits. Sealed: it names crate
/// internals, and nothing outside this crate may implement a query tuple.
pub(crate) mod sealed {
    use super::{Archetype, Entity, TypeId};

    pub trait SelectionRows {
        /// The rows of one archetype, front to back.
        type Rows<'w>: Iterator<Item = (Entity, Self::RowItem<'w>)>;
        /// The tuple of fetched items for one row.
        type RowItem<'w>;

        /// The component of every element, in tuple order.
        fn component_ids() -> Vec<TypeId>;

        /// Borrows the columns of `archetype`, or `None` when it lacks one.
        /// The caller has checked that `component_ids` are pairwise distinct.
        fn rows_mut(archetype: &mut Archetype) -> Option<Self::Rows<'_>>;
    }

    pub trait ReadSelectionRows: SelectionRows {
        /// Same as [`SelectionRows::rows_mut`] through a shared borrow.
        fn rows(archetype: &Archetype) -> Option<Self::Rows<'_>>;
    }
}
