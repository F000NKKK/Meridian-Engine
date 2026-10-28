//! The world clock: a counter that stamps when a column last changed.

/// A point on the world's clock. [`World::advance_tick`](crate::World::advance_tick)
/// moves it forward; a column remembers the tick of its last change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(u64);

impl Tick {
    /// Before anything happened: `Changed<T>` against it matches every
    /// column that was ever written.
    pub const ZERO: Tick = Tick(0);

    pub(crate) const FIRST: Tick = Tick(1);

    pub(crate) fn next(self) -> Tick {
        Tick(self.0 + 1)
    }

    /// The raw counter, e.g. to log or store it.
    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}
