//! [`GlobalTransform`]: a transform resolved through the hierarchy.

use meridian_gac::{Motor3, Vec3};

use crate::Component;

/// An entity's motor and scale in the world frame: its
/// [`Transform`](crate::Transform) composed with the global transforms of its
/// ancestors. Written by [`propagate_transforms`](crate::propagate_transforms),
/// read by systems that need a world position (rendering, audio, physics).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalTransform {
    /// The motor from the entity's frame to the world frame.
    pub motor: Motor3,
    /// The entity's size along its own axes in the world: the product of the
    /// scales along its chain of ancestors. A parent turned relative to a
    /// non-uniformly scaled child would shear it; the hierarchy keeps only
    /// this per-axis product instead (as most engines do).
    pub scale: Vec3,
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self {
            motor: Motor3::identity(),
            scale: Vec3::ONE,
        }
    }
}

impl Component for GlobalTransform {}
