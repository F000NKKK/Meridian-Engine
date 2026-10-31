//! A procedural icosphere mesh.

use meridian_gac::icosphere;

use crate::submission::MeshSource;

/// Builds a [`MeshSource`] for an icosphere of the given `radius`,
/// centered at its own local origin (world placement is
/// `Renderable3d::frame`'s job — `Motor3` has no scale component, so
/// radius has to be baked into the mesh itself, not applied per
/// instance). Normals are the unit-length vertex directions, UVs an
/// equirectangular projection. Seam/pole vertices are split for interpolation;
/// U can exceed 1 and requires repeat addressing. V=0 is the +Y pole.
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
    let mut source = MeshSource {
        positions,
        normals,
        uvs,
        indices,
        skin: None,
        morph: None,
    };
    unwrap_seam(&mut source);
    source
}

/// Each triangle interpolates across the short U interval, never across the map.
fn unwrap_seam(mesh: &mut MeshSource) {
    let mut next_index = mesh
        .indices
        .iter()
        .copied()
        .max()
        .map_or(0, |index| index + 1);
    for triangle in mesh.indices.as_chunks_mut::<3>().0 {
        let corners = [triangle[0], triangle[1], triangle[2]];
        let coordinates = corners.map(|index| mesh.uvs[index as usize]);
        let poles = corners.map(|index| mesh.normals[index as usize][1].abs() > 1.0 - 1e-6);
        for (index, uv) in triangle.iter_mut().zip(unwrapped(coordinates, poles)) {
            let original = *index as usize;
            if uv.map(f32::to_bits) != mesh.uvs[original].map(f32::to_bits) {
                *index = next_index;
                next_index += 1;
                mesh.positions.push(mesh.positions[original]);
                mesh.normals.push(mesh.normals[original]);
                mesh.uvs.push(uv);
            }
        }
    }
}

fn unwrapped(mut coordinates: [[f32; 2]; 3], poles: [bool; 3]) -> [[f32; 2]; 3] {
    let first = coordinates
        .iter()
        .zip(poles)
        .find_map(|(uv, pole)| (!pole).then_some(uv[0]))
        .unwrap_or(0.5);
    for (uv, pole) in coordinates.iter_mut().zip(poles) {
        if pole {
            uv[0] = first;
        }
    }
    let mut sorted = coordinates.map(|uv| uv[0]);
    sorted.sort_by(f32::total_cmp);
    let mut largest_gap = sorted[0] + 1.0 - sorted[2];
    let mut cut = sorted[0];
    for pair in sorted.windows(2) {
        if pair[1] - pair[0] > largest_gap {
            largest_gap = pair[1] - pair[0];
            cut = pair[1];
        }
    }
    for uv in &mut coordinates {
        if uv[0] < cut {
            uv[0] += 1.0;
        }
    }
    let (mut sum, mut count) = (0.0, 0.0);
    for (uv, pole) in coordinates.iter().zip(poles) {
        if !pole {
            sum += uv[0];
            count += 1.0;
        }
    }
    for (uv, pole) in coordinates.iter_mut().zip(poles) {
        if pole && count > 0.0 {
            uv[0] = sum / count;
        }
    }
    coordinates
}
