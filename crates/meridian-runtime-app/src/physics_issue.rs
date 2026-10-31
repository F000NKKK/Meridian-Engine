//! Diagnostics at the ECS/physics boundary.

use crate::PhysicsProblem;
use meridian_ecs::Entity;

/// An entity rejected by the ECS physics bridge; it was not simulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsIssue {
    /// The invalid entity.
    pub entity: Entity,
    /// The rejected configuration.
    pub problem: PhysicsProblem,
}
