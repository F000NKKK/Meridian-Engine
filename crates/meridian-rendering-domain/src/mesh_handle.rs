//! Typed handles for meshes.

use meridian_resource::ResourceId;

/// Marker type distinguishing mesh resource identifiers.
#[derive(Debug)]
pub struct MeshMarker;

/// Generational resource identifier for a mesh.
pub type MeshHandle = ResourceId<MeshMarker>;
