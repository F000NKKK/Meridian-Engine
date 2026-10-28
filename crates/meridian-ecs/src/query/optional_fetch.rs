//! `Option<&T>` and `Option<&mut T>`: a component an entity may lack.

use std::slice::{Iter, IterMut};

use super::optional_rows::OptionalRows;
use super::{Fetch, ReadFetch};
use crate::{Component, Tick};

impl<T: Component> Fetch for Option<&T> {
    type Component = T;
    type Item<'w> = Option<&'w T>;
    type Iter<'w> = OptionalRows<Iter<'w, T>>;

    const ACCESSES_COLUMN: bool = true;

    fn admits(_column: Option<Tick>, _since: Tick) -> bool {
        true
    }

    fn iter(column: Option<&mut [T]>, rows: usize) -> Self::Iter<'_> {
        OptionalRows::new(column.map(|column| column.iter()), rows)
    }
}

impl<T: Component> ReadFetch for Option<&T> {
    fn iter_shared(column: Option<&[T]>, rows: usize) -> Self::Iter<'_> {
        OptionalRows::new(column.map(<[T]>::iter), rows)
    }
}

impl<T: Component> Fetch for Option<&mut T> {
    type Component = T;
    type Item<'w> = Option<&'w mut T>;
    type Iter<'w> = OptionalRows<IterMut<'w, T>>;

    const ACCESSES_COLUMN: bool = true;
    const WRITES: bool = true;

    fn admits(_column: Option<Tick>, _since: Tick) -> bool {
        true
    }

    fn iter(column: Option<&mut [T]>, rows: usize) -> Self::Iter<'_> {
        OptionalRows::new(column.map(<[T]>::iter_mut), rows)
    }
}
