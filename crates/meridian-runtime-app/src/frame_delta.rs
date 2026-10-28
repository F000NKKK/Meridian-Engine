//! How long the last frame took, for stages that move with real time.

/// Seconds of real time the current frame covers. The composition root sets
/// it once a frame, before ticking the runtime, in a [`Shared`](crate::Shared)
/// handle that the stages needing it (animation) hold.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameDelta(pub f32);
