//! [`GlobalTransform`]: a transform resolved through the hierarchy.

use meridian_gac::{Motor3, Scale3};

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
    /// [`Scale3`] components along its chain of ancestors (the unit scale for
    /// an entity without one). A parent turned relative to a non-uniformly
    /// scaled child would shear it; the hierarchy keeps only this per-axis
    /// product instead (as most engines do).
    pub scale: Scale3,
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self {
            motor: Motor3::identity(),
            scale: Scale3::UNIT,
        }
    }
}

impl Component for GlobalTransform {}
