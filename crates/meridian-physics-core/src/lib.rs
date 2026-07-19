//! Rigid body dynamics built on the GAC: broad/narrow phase collision, constraint solving and integration.

use meridian_gac_core::Motor3;
use meridian_resource_core::ResourceId;

/// Marker type for collider-mesh `ResourceId`s — see
/// docs/adr/006-resource-core-separation.md.
pub struct ColliderMeshMarker;
pub type ColliderMeshHandle = ResourceId<ColliderMeshMarker>;

/// A simulated rigid body: spatial frame (shared with every other
/// subsystem via the GAC) + linear state.
#[derive(Debug, Clone, Copy, Default)]
pub struct RigidBody {
    pub frame: Motor3,
    pub velocity: [f32; 3],
    pub mass: f32,
}

/// A broad-phase acceleration structure (BVH, spatial hash, ...). Owned
/// here, not in `physics-driver` — see docs/physics-design.md.
#[derive(Debug, Default)]
pub struct BroadPhase;

/// A single narrow-phase contact between two colliders.
#[derive(Debug, Clone, Copy, Default)]
pub struct Contact;

/// Resolves contacts into corrective impulses.
#[derive(Debug, Default)]
pub struct ConstraintSolver;
