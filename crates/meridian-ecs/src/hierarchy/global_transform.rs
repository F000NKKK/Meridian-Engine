//! [`GlobalTransform`]: a transform resolved through the hierarchy.

use meridian_gac::Motor3;

use crate::Component;

/// An entity's motor in the world frame: its [`Transform`](crate::Transform)
/// composed with the global motors of its ancestors. Written by
/// [`propagate_transforms`](crate::propagate_transforms), read by systems that
/// need a world position (rendering, audio, physics).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GlobalTransform {
    /// The motor from the entity's frame to the world frame.
    pub motor: Motor3,
}

impl Component for GlobalTransform {}
