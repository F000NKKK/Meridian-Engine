//! CPU-side mesh geometry awaiting upload.

/// The CPU-side source a [`MeshHandle`](crate::MeshHandle) resolves to: local-space
/// positions, per-vertex normals and UVs, and triangle indices, decoded
/// elsewhere (typically `asset-domain::ObjDecoder`/glTF once that lands)
/// and registered here once. `normals.len()`/`uvs.len()` must equal
/// `positions.len()`, `indices.len()` must be a multiple of 3 (triangle
/// list), and every index must name a vertex. Indices are `u32` end to
/// end, so a mesh may have more than 65535 vertices (see
/// [`MeshRegistry::register`](crate::submission::MeshRegistry::register)).
#[derive(Debug, Clone)]
pub struct MeshSource {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}
