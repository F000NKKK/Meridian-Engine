//! `With<T>` and `Without<T>`: keep or drop entities by a component they carry no data from.

use std::iter::{RepeatN, repeat_n};
use std::marker::PhantomData;

use super::{Fetch, ReadFetch};
use crate::Component;

/// Query element that matches entities that have a `T`, without reading it.
/// Yields `()` per row; never constructed.
#[derive(Debug)]
pub struct With<T>(PhantomData<fn() -> T>);

/// Query element that matches entities that do not have a `T`.
/// Yields `()` per row; never constructed.
#[derive(Debug)]
pub struct Without<T>(PhantomData<fn() -> T>);

impl<T: Component> Fetch for With<T> {
    type Component = T;
    type Item<'w> = ();
    type Iter<'w> = RepeatN<()>;

    const ACCESSES_COLUMN: bool = false;

    fn admits(has_component: bool) -> bool {
        has_component
    }

    fn iter(_column: Option<&mut [T]>, rows: usize) -> RepeatN<()> {
        repeat_n((), rows)
    }
}

impl<T: Component> ReadFetch for With<T> {
    fn iter_shared(_column: Option<&[T]>, rows: usize) -> RepeatN<()> {
        repeat_n((), rows)
    }
}

impl<T: Component> Fetch for Without<T> {
    type Component = T;
    type Item<'w> = ();
    type Iter<'w> = RepeatN<()>;

    const ACCESSES_COLUMN: bool = false;

    fn admits(has_component: bool) -> bool {
        !has_component
    }

    fn iter(_column: Option<&mut [T]>, rows: usize) -> RepeatN<()> {
        repeat_n((), rows)
    }
}

impl<T: Component> ReadFetch for Without<T> {
    fn iter_shared(_column: Option<&[T]>, rows: usize) -> RepeatN<()> {
        repeat_n((), rows)
    }
}
