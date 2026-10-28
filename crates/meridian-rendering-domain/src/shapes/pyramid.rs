//! A procedural square-based pyramid mesh.

use meridian_gac::Vec3;

use crate::submission::MeshSource;

/// A square-based pyramid: base half-extent `base_half_extent`, apex
/// `height` above the base, centered on the base's own middle (so a
/// `Renderable3d::frame` translation places the base center, not the
/// centroid). Flat per-face normals (5 faces: 1 base quad + 4 triangular
/// sides), windings verified the same way as [`cube_mesh_source`](super::cube_mesh_source)'s —
/// see that function's doc comment for why this matters.
#[must_use]
pub fn pyramid_mesh_source(base_half_extent: f32, height: f32) -> MeshSource {
    let base = [
        [-base_half_extent, 0.0, -base_half_extent],
        [base_half_extent, 0.0, -base_half_extent],
        [base_half_extent, 0.0, base_half_extent],
        [-base_half_extent, 0.0, base_half_extent],
    ];
    let apex = [0.0f32, height, 0.0];

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Base quad, normal -Y, triangles (0,1,2)/(0,2,3).
    {
        let base_idx = positions.len() as u32;
        for corner in base {
            positions.push(corner);
            normals.push([0.0, -1.0, 0.0]);
        }
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx,
            base_idx + 2,
            base_idx + 3,
        ]);
    }

    // Four triangular sides: (base[i], apex, base[i+1]) — verified
    // outward-facing winding, see the module-level comment above.
    let side_uvs = [[0.0, 0.0], [0.5, 1.0], [1.0, 0.0]];
    for i in 0..4 {
        let b0 = base[i];
        let b1 = base[(i + 1) % 4];
        let normal = triangle_normal(b0, apex, b1);
        let base_idx = positions.len() as u32;
        for corner in [b0, apex, b1] {
            positions.push(corner);
            normals.push(normal);
        }
        uvs.extend_from_slice(&side_uvs);
        indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);
    }

    MeshSource {
        positions,
        normals,
        uvs,
        indices,
        skin: None,
    }
}

/// The unit normal of triangle `(origin, towards, then)`: the cross
/// product of the edges `origin -> towards` and `origin -> then`.
fn triangle_normal(origin: [f32; 3], towards: [f32; 3], then: [f32; 3]) -> [f32; 3] {
    let point = |p: [f32; 3]| Vec3::new(p[0], p[1], p[2]);
    let normal = (point(towards) - point(origin))
        .cross(point(then) - point(origin))
        .normalize();
    [normal.x, normal.y, normal.z]
}
