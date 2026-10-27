//! How one tuple element (`&T` or `&mut T`) walks a component column.

use std::slice::{Iter, IterMut};

use crate::Component;

/// One element of a multi-component query: `&T` reads a column, `&mut T`
/// writes it. Implemented for exactly those two shapes; not extensible.
pub trait Fetch: sealed::Sealed {
    /// The component type whose column this element walks.
    type Component: Component;
    /// What one row yields: `&'w T` or `&'w mut T`.
    type Item<'w>;
    /// The iterator over one archetype's column.
    type Iter<'w>: Iterator<Item = Self::Item<'w>>;

    /// Walks `column` front to back.
    fn iter(column: &mut [Self::Component]) -> Self::Iter<'_>;
}

/// A [`Fetch`] that only reads, so it can run on a shared `&World`.
pub trait ReadFetch: Fetch {
    /// Walks `column` front to back through a shared borrow.
    fn iter_shared(column: &[Self::Component]) -> Self::Iter<'_>;
}

impl<T: Component> Fetch for &T {
    type Component = T;
    type Item<'w> = &'w T;
    type Iter<'w> = Iter<'w, T>;

    fn iter(column: &mut [T]) -> Iter<'_, T> {
        column.iter()
    }
}

impl<T: Component> ReadFetch for &T {
    fn iter_shared(column: &[T]) -> Iter<'_, T> {
        column.iter()
    }
}

impl<T: Component> Fetch for &mut T {
    type Component = T;
    type Item<'w> = &'w mut T;
    type Iter<'w> = IterMut<'w, T>;

    fn iter(column: &mut [T]) -> IterMut<'_, T> {
        column.iter_mut()
    }
}

mod sealed {
    pub trait Sealed {}
    impl<T> Sealed for &T {}
    impl<T> Sealed for &mut T {}
}
