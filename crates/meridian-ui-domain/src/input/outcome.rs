//! Whether the UI took an input event.

/// What happened to an [`crate::InputEvent`]. Gameplay reads this to leave
/// input the UI used alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum InputOutcome {
    /// The UI used the event; gameplay must not.
    Consumed,
    /// Nothing in the UI wanted it.
    Ignored,
}
