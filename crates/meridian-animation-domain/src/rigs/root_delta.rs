//! How far an animation carried a model in one step.

use std::ops::{Add, Mul};

use meridian_gac::Vec3;

/// The ground an animation covered in a step: the travel of the root joint
/// and, when the root motion follows it, how far it turned.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RootDelta {
    /// Horizontal travel in the model's own frame as it faced at the start of
    /// the step; y is zero.
    pub translation: Vec3,
    /// Turn about the vertical axis in radians, positive counter-clockwise
    /// seen from above (right-handed about +y).
    pub yaw: f32,
}

impl Add for RootDelta {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            translation: self.translation + other.translation,
            yaw: self.yaw + other.yaw,
        }
    }
}

impl Mul<f32> for RootDelta {
    type Output = Self;

    fn mul(self, weight: f32) -> Self {
        Self {
            translation: self.translation * weight,
            yaw: self.yaw * weight,
        }
    }
}
