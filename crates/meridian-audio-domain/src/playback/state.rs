//! Where a voice is in its life.

/// Where a [`Voice`](super::Voice) is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    /// Pulling samples from its source.
    Playing,
    /// Silent; the source keeps its position.
    Paused,
    /// Ended (the source ran out, or [`Voice::stop`](super::Voice::stop) was called); stays silent
    /// once any fade-out finishes.
    Finished,
}
