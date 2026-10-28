//! A procedural ground plane mesh.

use crate::submission::MeshSource;

/// A flat quad in the local `y = 0` plane, `half_size` from center to
/// edge, its UVs tiled `uv_tiles` times across — the floor's mesh, with
/// world placement left to `Renderable3d::frame`.
#[must_use]
pub fn ground_mesh_source(half_size: f32, uv_tiles: f32) -> MeshSource {
    MeshSource {
        positions: vec![
            [-half_size, 0.0, -half_size],
            [half_size, 0.0, -half_size],
            [half_size, 0.0, half_size],
            [-half_size, 0.0, half_size],
        ],
        normals: vec![[0.0, 1.0, 0.0]; 4],
        uvs: vec![
            [0.0, 0.0],
            [uv_tiles, 0.0],
            [uv_tiles, uv_tiles],
            [0.0, uv_tiles],
        ],
        // Winding front-facing from +Y — verified the same way as
        // `cube_mesh_source`'s faces.
        indices: vec![0, 2, 1, 0, 3, 2],
        skin: None,
    }
}
