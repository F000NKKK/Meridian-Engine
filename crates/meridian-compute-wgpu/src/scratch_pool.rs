//! Scratch state shared by repeated, possibly overlapping, dispatches.

use std::sync::{Mutex, PoisonError};

/// A free list of scratch values (buffers plus the bind group over them).
/// A dispatch [`take`](Self::take)s one, uses it across its `await`s and
/// [`put`](Self::put)s it back; two dispatches in flight at once each get their
/// own, so they never write into each other's buffers, and a kernel dispatched
/// once per frame keeps reusing a single one.
#[derive(Debug)]
pub struct ScratchPool<S> {
    free: Mutex<Vec<S>>,
}

impl<S> Default for ScratchPool<S> {
    fn default() -> Self {
        Self {
            free: Mutex::new(Vec::new()),
        }
    }
}

impl<S: Default> ScratchPool<S> {
    /// An empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A free scratch value, or a new one if every other is in use.
    pub fn take(&self) -> S {
        self.free
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop()
            .unwrap_or_default()
    }

    /// Returns `scratch` for the next dispatch.
    pub fn put(&self, scratch: S) {
        self.free
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(scratch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_returned_value_is_the_one_taken_next() {
        let pool = ScratchPool::<Vec<u8>>::new();
        let mut scratch = pool.take();
        scratch.push(7);
        pool.put(scratch);
        assert_eq!(pool.take(), [7]);
    }

    #[test]
    fn overlapping_takes_get_separate_values() {
        let pool = ScratchPool::<Vec<u8>>::new();
        let mut first = pool.take();
        first.push(1);
        let second = pool.take();
        assert_eq!(second, Vec::new());
        pool.put(first);
        pool.put(second);
    }
}
