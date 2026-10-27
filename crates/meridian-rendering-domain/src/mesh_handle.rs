//! Typed handles for meshes.

use meridian_resource::ResourceId;

#[derive(Debug)]
pub struct MeshMarker;

pub type MeshHandle = ResourceId<MeshMarker>;
