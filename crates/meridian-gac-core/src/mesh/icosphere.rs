//! Procedural icosphere generation.

use super::{Face, Mesh};
use crate::flavor::{GaFlavor, ScalarLike, VectorLike};

/// A unit-radius icosphere (centered at the origin — callers translate/
/// scale as needed): a `subdivisions`-times-subdivided icosahedron (`0`
/// = the base 12-vertex icosahedron, each further level roughly
/// quadruples the triangle count), the standard "start from a regular
/// icosahedron, repeatedly split each triangle into 4 by its edge
/// midpoints re-normalized to the unit sphere" construction. The base 12
/// vertices are all even permutations of `(0, ±1, ±φ)` with the golden
/// ratio `φ`, normalized.
pub fn icosphere<F: GaFlavor>(subdivisions: u32) -> Mesh<F> {
    let mut vertices = base_vertices::<F>();
    let mut faces = base_faces();

    for _ in 0..subdivisions {
        let mut midpoint_cache = std::collections::HashMap::new();
        let mut next_faces = Vec::with_capacity(faces.len() * 4);
        for face in &faces {
            let (a, b, c) = (face.indices[0], face.indices[1], face.indices[2]);
            let ab = icosphere_midpoint::<F>(&mut vertices, &mut midpoint_cache, a, b);
            let bc = icosphere_midpoint::<F>(&mut vertices, &mut midpoint_cache, b, c);
            let ca = icosphere_midpoint::<F>(&mut vertices, &mut midpoint_cache, c, a);
            next_faces.push(Face::triangle(a, ab, ca));
            next_faces.push(Face::triangle(b, bc, ab));
            next_faces.push(Face::triangle(c, ca, bc));
            next_faces.push(Face::triangle(ab, bc, ca));
        }
        faces = next_faces;
    }

    Mesh { vertices, faces }
}

/// The 12 normalized vertices of the base icosahedron.
fn base_vertices<F: GaFlavor>() -> Vec<F::Vector> {
    let phi =
        (F::Scalar::ONE + F::Scalar::from_f64(5.0).sqrt()) / (F::Scalar::ONE + F::Scalar::ONE);
    let raw: [[F::Scalar; 3]; 12] = {
        let z = F::Scalar::ZERO;
        let o = F::Scalar::ONE;
        [
            [-o, phi, z],
            [o, phi, z],
            [-o, -phi, z],
            [o, -phi, z],
            [z, -o, phi],
            [z, o, phi],
            [z, -o, -phi],
            [z, o, -phi],
            [phi, z, -o],
            [phi, z, o],
            [-phi, z, -o],
            [-phi, z, o],
        ]
    };
    raw.iter()
        .map(|v| F::Vector::new(v[0], v[1], v[2]).normalize())
        .collect()
}

/// The 20 triangular faces of the base icosahedron.
fn base_faces() -> Vec<Face> {
    vec![
        Face::triangle(0, 11, 5),
        Face::triangle(0, 5, 1),
        Face::triangle(0, 1, 7),
        Face::triangle(0, 7, 10),
        Face::triangle(0, 10, 11),
        Face::triangle(1, 5, 9),
        Face::triangle(5, 11, 4),
        Face::triangle(11, 10, 2),
        Face::triangle(10, 7, 6),
        Face::triangle(7, 1, 8),
        Face::triangle(3, 9, 4),
        Face::triangle(3, 4, 2),
        Face::triangle(3, 2, 6),
        Face::triangle(3, 6, 8),
        Face::triangle(3, 8, 9),
        Face::triangle(4, 9, 5),
        Face::triangle(2, 4, 11),
        Face::triangle(6, 2, 10),
        Face::triangle(8, 6, 7),
        Face::triangle(9, 8, 1),
    ]
}

/// Returns the index of the unit-sphere midpoint between vertices `i`/`j`
/// — reused from `cache` if this edge was already split (shared between
/// two adjacent triangles), otherwise computed and cached. Helper for
/// [`icosphere`]'s subdivision step.
fn icosphere_midpoint<F: GaFlavor>(
    vertices: &mut Vec<F::Vector>,
    cache: &mut std::collections::HashMap<(usize, usize), usize>,
    i: usize,
    j: usize,
) -> usize {
    let key = if i < j { (i, j) } else { (j, i) };
    if let Some(&existing) = cache.get(&key) {
        return existing;
    }
    let mid = (vertices[i] + vertices[j]).normalize();
    let index = vertices.len();
    vertices.push(mid);
    cache.insert(key, index);
    index
}
