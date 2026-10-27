//! A procedural cube mesh.

use crate::submission::MeshSource;

type CubeFace = ([f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]);

const CUBE_FACES: [CubeFace; 6] = [
    // (normal, corner00, corner10, corner11, corner01) — CCW as seen
    // from outside the cube along `normal`.
    (
        [1.0, 0.0, 0.0],
        [1.0, -1.0, -1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, -1.0],
    ),
    (
        [-1.0, 0.0, 0.0],
        [-1.0, -1.0, 1.0],
        [-1.0, -1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, 1.0, 1.0],
    ),
    (
        [0.0, 1.0, 0.0],
        [-1.0, 1.0, -1.0],
        [1.0, 1.0, -1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ),
    (
        [0.0, -1.0, 0.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, -1.0, -1.0],
        [-1.0, -1.0, -1.0],
    ),
    // +Z and -Z: reversed corner order relative to the naive pattern
    // above — these two were the ones caught inside-out (verified by
    // computing cross(edge1, edge2)·normal by hand for all six faces;
    // only +Z/-Z came out negative with the naive order).
    (
        [0.0, 0.0, 1.0],
        [-1.0, -1.0, 1.0],
        [-1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, -1.0, 1.0],
    ),
    (
        [0.0, 0.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, -1.0],
    ),
];

/// A cube with half-extent `half_extent`, one set of 4 vertices per face
/// so each face gets its own flat normal and a full `[0,1]` UV. Winding
/// verified per-face (not just assumed from a formula) — every face's
/// `cross(edge1, edge2)` must point along its own outward normal for
/// `rendering-wgpu`'s `FrontFace::Ccw` + back-face culling to actually
/// draw it, and a mirrored-looking cube (some faces visible only from
/// inside) is exactly what one flipped face produces.
#[must_use]
pub fn cube_mesh_source(half_extent: f32) -> MeshSource {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for (normal, c00, c10, c11, c01) in CUBE_FACES {
        let base = positions.len() as u32;
        for corner in [c00, c10, c11, c01] {
            positions.push(corner.map(|c| c * half_extent));
            normals.push(normal);
        }
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    MeshSource {
        positions,
        normals,
        uvs,
        indices,
    }
}
