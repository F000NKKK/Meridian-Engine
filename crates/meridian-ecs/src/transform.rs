//! The spatial component: a motor and a scale.

use meridian_gac::{Motor3, Vec3};

use crate::Component;

/// The engine-wide spatial component: rotation + translation as a single
/// motor, shared with physics/graphics/audio (see docs/gac-design.md), and
/// a per-axis scale a motor cannot hold.
///
/// The scale applies first, in the entity's own axes, then the motor places
/// the result in the parent's frame (`T * R * S`). A child under a scaled
/// parent is scaled and displaced by it; see
/// [`propagate_transforms`](crate::propagate_transforms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Rotation and translation, placing the scaled entity in its parent's frame.
    pub motor: Motor3,
    /// The size along each local axis; `Vec3::ONE` leaves the entity as is.
    pub scale: Vec3,
}

impl Transform {
    /// A transform that places by `motor` and does not scale.
    #[must_use]
    pub const fn from_motor(motor: Motor3) -> Self {
        Self {
            motor,
            scale: Vec3::ONE,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::from_motor(Motor3::identity())
    }
}

impl Component for Transform {}
