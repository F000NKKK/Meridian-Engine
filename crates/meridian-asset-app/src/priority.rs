//! How soon a background job should run.

/// The order in which queued loads start: every [`High`](Self::High) job
/// before any [`Normal`](Self::Normal) one, and those before any
/// [`Low`](Self::Low) one; jobs of equal priority start in the order they
/// were queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Priority {
    /// Prefetch and other work nobody waits for.
    Low,
    /// The default.
    #[default]
    Normal,
    /// Something the player is waiting for.
    High,
}
