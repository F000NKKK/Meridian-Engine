//! Typed handle for collider meshes.

use meridian_resource::ResourceId;

/// Marker type for collider-mesh `ResourceId`s — see
/// docs/adr/006-resource-core-separation.md.
#[derive(Debug)]
pub struct ColliderMeshMarker;

/// Generational resource identifier for a collider mesh.
pub type ColliderMeshHandle = ResourceId<ColliderMeshMarker>;
