//! Typed handles for meshes.

use meridian_resource_core::ResourceId;

#[derive(Debug)]
pub struct MeshMarker;

pub type MeshHandle = ResourceId<MeshMarker>;
