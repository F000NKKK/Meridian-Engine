//! CPU-side mesh geometry awaiting upload.

/// The CPU-side source a [`MeshHandle`](crate::MeshHandle) resolves to: local-space
/// positions, per-vertex normals and UVs, and triangle indices, decoded
/// elsewhere (typically `asset-core::ObjDecoder`/glTF once that lands)
/// and registered here once. `normals.len()`/`uvs.len()` must equal
/// `positions.len()`, `indices.len()` must be a multiple of 3 (triangle
/// list), and `positions.len()` must fit in `u16` — `graphics-driver`
/// only supports `u16` index buffers today (see
/// `meridian_rendering_wgpu::RenderPass::set_index_buffer_u16`'s own
/// doc comment); meshes beyond 65535 vertices are a future `u32`-index
/// follow-up, not something this bridge silently mishandles (see
/// [`MeshRegistry::register`](crate::submission::MeshRegistry::register)).
#[derive(Debug, Clone)]
pub struct MeshSource {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}
