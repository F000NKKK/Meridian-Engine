//! An emitter: a sound source's frame in the world.

use meridian_gac::{Motor3, Vec3};

/// A sound source's spatial frame. Playback state lives in `Voice`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Emitter {
    /// Position and orientation of the sound source.
    pub frame: Motor3,
}

impl Emitter {
    /// Returns the source's world-space position.
    pub fn position(&self) -> Vec3 {
        self.frame.transform_point(Vec3::ZERO)
    }
}
