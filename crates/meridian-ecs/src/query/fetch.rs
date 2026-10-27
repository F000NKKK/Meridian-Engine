//! How one tuple element (`&T`, `&mut T`, `Option<&T>`, `With<T>`, ..) walks a component column.

use std::slice::{Iter, IterMut};

use super::sealed::Sealed;
use crate::{Component, Tick};

/// One element of a multi-component query. Implemented for exactly these
/// shapes (not extensible):
///
/// | element | matches an archetype that | yields |
/// |---|---|---|
/// | `&T` | has `T` | `&T` |
/// | `&mut T` | has `T` | `&mut T` |
/// | `Option<&T>` | may or may not have `T` | `Option<&T>` |
/// | `Option<&mut T>` | may or may not have `T` | `Option<&mut T>` |
/// | [`With<T>`](crate::With) | has `T` | `()` |
/// | [`Without<T>`](crate::Without) | lacks `T` | `()` |
pub trait Fetch: Sealed {
    /// The component type this element looks at.
    type Component: Component;
    /// What one row yields.
    type Item<'w>;
    /// The iterator over one matching archetype's rows.
    type Iter<'w>: Iterator<Item = Self::Item<'w>>;

    /// Whether the element reads or writes the component's column. `false`
    /// for the filters, which only look at the archetype's component set.
    const ACCESSES_COLUMN: bool;

    /// Whether the element writes the column, so the column counts as
    /// changed once a query hands it out.
    const WRITES: bool = false;

    /// Whether an archetype matches, given the tick its component column
    /// last changed at (`None` when it has no such column) and the tick the
    /// query looks back to.
    fn admits(column: Option<Tick>, since: Tick) -> bool;

    /// Walks `rows` rows of `column` (`None` when the archetype has no such
    /// column or the element does not access it).
    fn iter(column: Option<&mut [Self::Component]>, rows: usize) -> Self::Iter<'_>;
}

/// A [`Fetch`] that only reads, so it can run on a shared `&World`.
pub trait ReadFetch: Fetch {
    /// Walks `rows` rows of `column` through a shared borrow.
    fn iter_shared(column: Option<&[Self::Component]>, rows: usize) -> Self::Iter<'_>;
}

impl<T: Component> Fetch for &T {
    type Component = T;
    type Item<'w> = &'w T;
    type Iter<'w> = Iter<'w, T>;

    const ACCESSES_COLUMN: bool = true;

    fn admits(column: Option<Tick>, _since: Tick) -> bool {
        column.is_some()
    }

    fn iter(column: Option<&mut [T]>, _rows: usize) -> Iter<'_, T> {
        column.unwrap_or_default().iter()
    }
}

impl<T: Component> ReadFetch for &T {
    fn iter_shared(column: Option<&[T]>, _rows: usize) -> Iter<'_, T> {
        column.unwrap_or_default().iter()
    }
}

impl<T: Component> Fetch for &mut T {
    type Component = T;
    type Item<'w> = &'w mut T;
    type Iter<'w> = IterMut<'w, T>;

    const ACCESSES_COLUMN: bool = true;
    const WRITES: bool = true;

    fn admits(column: Option<Tick>, _since: Tick) -> bool {
        column.is_some()
    }

    fn iter(column: Option<&mut [T]>, _rows: usize) -> IterMut<'_, T> {
        column.unwrap_or_default().iter_mut()
    }
}
