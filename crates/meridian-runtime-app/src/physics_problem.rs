//! Rejected ECS physics configurations.

/// Configurations unsupported by the initial rigid-body bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsProblem {
    /// Physics currently requires root entities with unit scale.
    ParentOrScale,
    /// No finite local rigid pose could be resolved.
    Pose,
    /// Mass, velocity or collider dimensions are invalid.
    Parameters,
}
