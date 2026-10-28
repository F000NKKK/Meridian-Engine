//! What playback does at the end of a clip.

/// What happens when playback reaches the end of the clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    /// Stops on the last frame.
    Once,
    /// Starts again from the beginning.
    #[default]
    Loop,
}
