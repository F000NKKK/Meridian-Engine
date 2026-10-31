//! The listener: the ear's frame in the world.

use meridian_gac::{Motor3, Vec3};

/// The listener's spatial frame for 3D audio.
#[derive(Debug, Clone, Copy, Default)]
pub struct Listener {
    /// Position and orientation of the listener.
    pub frame: Motor3,
}

impl Listener {
    /// Returns the listener's world-space position.
    pub fn position(&self) -> Vec3 {
        self.frame.transform_point(Vec3::ZERO)
    }
}
