//! [`UnknownComponents`]: what loading does with a name the registry lacks.

/// What loading does with a component name the registry doesn't know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownComponents {
    /// An error, with the closest registered name suggested: what a game
    /// loading its own content wants.
    #[default]
    Reject,
    /// Kept as written and saved back untouched: what an editor wants, so a
    /// scene written by a newer game still opens.
    Keep,
}
