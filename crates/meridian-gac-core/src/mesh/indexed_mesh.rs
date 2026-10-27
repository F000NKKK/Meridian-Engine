//! An indexed polygon mesh, generic over the flavor.

use super::Face;
use crate::flavor::GaFlavor;

/// An indexed polygon mesh: vertex positions (in some [`GaFlavor`]) plus
/// face topology (flavor-independent). Shared by any subsystem that
/// needs mesh data as *geometry* — `physics-core`'s soft-body particle/
/// spring setup ([`icosphere`] specifically), a future `graphics-core`
/// procedural-mesh path, ... — the same "one primitive, reused, not
/// duplicated per consumer" reasoning as [`Aabb`]/[`Sphere`]/[`Shape`]
/// above. This is deliberately *not* the same type as
/// `asset-core::MeshAsset`: that type is always `f32`, always
/// pre-triangulated, decoded-from-file-bytes data already shaped for GPU
/// upload (see that crate's own module doc and
/// [dependency-rules.md](../../../docs/dependency-rules.md) rule 4);
/// `Mesh<F>` is for geometry *authored or generated as geometry* (a
/// procedural icosphere, a physics simulation's own topology), generic
/// over scalar flavor the way everything else in this module is.
#[derive(Debug, Clone)]
pub struct Mesh<F: GaFlavor> {
    pub vertices: Vec<F::Vector>,
    pub faces: Vec<Face>,
}

impl<F: GaFlavor> Mesh<F> {
    /// Every edge across every face's fan-triangulation
    /// ([`Face::triangles`]), each returned exactly once regardless of
    /// how many faces share it.
    pub fn unique_edges(&self) -> Vec<(usize, usize)> {
        let mut seen = std::collections::HashSet::new();
        let mut edges = Vec::new();
        for face in &self.faces {
            for (a, b, c) in face.triangles() {
                for (x, y) in [(a, b), (b, c), (c, a)] {
                    let key = if x < y { (x, y) } else { (y, x) };
                    if seen.insert(key) {
                        edges.push(key);
                    }
                }
            }
        }
        edges
    }
}
