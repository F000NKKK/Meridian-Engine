//! Dynamic motion state independent of collision geometry and spatial authoring.

use meridian_ecs::Component;
use meridian_gac::{Bivector3, Vec3};

/// Motion state for an entity with Collider. Positive mass is dynamic;
/// zero mass is static. The physics bridge updates velocities after each step.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PhysicsBody {
    /// Mass in kilograms; finite and nonnegative.
    pub mass: f32,
    /// World-space linear velocity in units per second.
    pub velocity: Vec3,
    /// Angular velocity as a GA bivector.
    pub angular_velocity: Bivector3,
}

impl Component for PhysicsBody {}
