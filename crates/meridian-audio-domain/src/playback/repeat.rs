//! What a looping source does at its end.

/// What a source does when it reaches the end of its samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    /// Play once, then end.
    Once,
    /// Wrap to the start and keep playing.
    Loop,
}
