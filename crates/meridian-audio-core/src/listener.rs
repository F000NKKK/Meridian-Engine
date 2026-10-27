//! The listener: the ear's frame in the world.

use meridian_gac_core::{Motor3, Vec3};

/// The listener's spatial frame for 3D audio.
#[derive(Debug, Clone, Copy, Default)]
pub struct Listener {
    pub frame: Motor3,
}

impl Listener {
    pub fn position(&self) -> Vec3 {
        self.frame.transform_point(Vec3::ZERO)
    }
}
