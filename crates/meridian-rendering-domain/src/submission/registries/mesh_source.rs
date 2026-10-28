//! CPU-side mesh geometry awaiting upload.

use meridian_gac::{Aabb, Vec3};

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

impl MeshSource {
    /// The local-space box around every vertex, `None` for a mesh without any.
    #[must_use]
    pub fn bounds(&self) -> Option<Aabb> {
        let mut points = self.positions.iter().map(|&[x, y, z]| Vec3::new(x, y, z));
        let first = points.next()?;
        let (min, max) = points.fold((first, first), |(min, max), p| {
            (
                Vec3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z)),
                Vec3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z)),
            )
        });
        Some(Aabb { min, max })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(positions: Vec<[f32; 3]>) -> MeshSource {
        MeshSource {
            positions,
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
        }
    }

    #[test]
    fn bounds_enclose_every_vertex() {
        let bounds = mesh(vec![[1.0, -2.0, 3.0], [-4.0, 5.0, 0.0]]).bounds();
        assert_eq!(
            bounds,
            Some(Aabb {
                min: Vec3::new(-4.0, -2.0, 0.0),
                max: Vec3::new(1.0, 5.0, 3.0),
            })
        );
    }

    #[test]
    fn a_mesh_without_vertices_has_no_bounds() {
        assert_eq!(mesh(Vec::new()).bounds(), None);
    }
}
