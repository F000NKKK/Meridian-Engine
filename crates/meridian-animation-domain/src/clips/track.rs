//! One animated property of a joint.

use meridian_gac::{Rotor, Vec3};

use super::Keys;
use crate::JointTransform;

/// The keyframes of one property of a joint's local transform.
#[derive(Debug, Clone, PartialEq)]
pub enum Track {
    /// Local translation, `[x, y, z]`.
    Translation(Keys<3>),
    /// Local rotation as a unit quaternion, `[x, y, z, w]`.
    Rotation(Keys<4>),
    /// Local scale, `[x, y, z]`.
    Scale(Keys<3>),
}

impl Track {
    /// The time of the track's last key.
    #[must_use]
    pub fn end_time(&self) -> f32 {
        match self {
            Self::Translation(keys) | Self::Scale(keys) => keys.end_time(),
            Self::Rotation(keys) => keys.end_time(),
        }
    }

    /// Sets the property this track animates in `transform` to its value at
    /// `time`, leaving the others alone.
    pub fn apply(&self, time: f32, transform: &mut JointTransform) {
        match self {
            Self::Translation(keys) => transform.translation = vec3(keys.sample(time)),
            Self::Rotation(keys) => transform.rotation = Rotor::from_quaternion(keys.sample(time)),
            Self::Scale(keys) => transform.scale = vec3(keys.sample(time)),
        }
    }
}

fn vec3([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(x, y, z)
}
