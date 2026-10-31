//! Diagnostics from resolving independently authored spatial components.

use crate::SpatialProblem;
use meridian_ecs::Entity;

/// A spatial authoring problem and the entity that owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpatialIssue {
    /// Entity whose derived Transform was left unchanged.
    pub entity: Entity,
    /// Why the source components could not be resolved.
    pub problem: SpatialProblem,
}
