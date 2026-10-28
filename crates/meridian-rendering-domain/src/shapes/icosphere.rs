//! A procedural icosphere mesh.

use meridian_gac::icosphere;

use crate::submission::MeshSource;

/// Builds a [`MeshSource`] for an icosphere of the given `radius`,
/// centered at its own local origin (world placement is
/// `Renderable3d::frame`'s job — `Motor3` has no scale component, so
/// radius has to be baked into the mesh itself, not applied per
/// instance). Normals are the unit-length vertex directions, UVs an
/// equirectangular projection.
#[must_use]
pub fn icosphere_mesh_source(subdivisions: u32, radius: f32) -> MeshSource {
    let mesh = icosphere(subdivisions);
    let positions: Vec<[f32; 3]> = mesh
        .vertices
        .iter()
        .map(|v| [v.x * radius, v.y * radius, v.z * radius])
        .collect();
    let normals: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| [v.x, v.y, v.z]).collect();
    let uvs: Vec<[f32; 2]> = mesh
        .vertices
        .iter()
        .map(|v| {
            let n = v.normalize();
            let u = 0.5 + n.z.atan2(n.x) / std::f32::consts::TAU;
            let t = 0.5 - n.y.asin() / std::f32::consts::PI;
            [u, t]
        })
        .collect();
    let mut indices = Vec::new();
    for face in &mesh.faces {
        for (a, b, c) in face.triangles() {
            indices.push(a as u32);
            indices.push(b as u32);
            indices.push(c as u32);
        }
    }
    MeshSource {
        positions,
        normals,
        uvs,
        indices,
        skin: None,
    }
}
