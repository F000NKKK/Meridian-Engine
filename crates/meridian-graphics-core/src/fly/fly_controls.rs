//! One frame of free-fly input.

/// What the player asked a [`FlyCamera`](super::FlyCamera) to do this frame,
/// independent of any input device. The windowing layer maps keys and the
/// mouse onto it (`meridian-sdk::fly_controls`), so the camera itself needs
/// no platform types.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FlyControls {
    /// Move along the way the camera faces (W).
    pub forward: bool,
    /// Move against the way the camera faces (S).
    pub backward: bool,
    /// Strafe left (A).
    pub left: bool,
    /// Strafe right (D).
    pub right: bool,
    /// Rise along world up (Space).
    pub up: bool,
    /// Sink along world up (Ctrl).
    pub down: bool,
    /// Move three times faster (Shift).
    pub boost: bool,
    /// Device-relative mouse motion `(dx, dy)` since the last frame.
    pub look: (f32, f32),
}
