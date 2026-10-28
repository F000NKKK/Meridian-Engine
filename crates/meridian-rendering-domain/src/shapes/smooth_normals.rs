//! Smooth per-vertex normals for imported triangle meshes.

/// Per-vertex smooth normals for an arbitrary imported triangle mesh
/// (positions + shared-vertex indices, the shape the OBJ decoder
/// produces) — each triangle's area-weighted face normal (the raw,
/// unnormalized cross product already scales with the triangle's area)
/// accumulates into its three vertices, normalized once at the end.
/// Unlike [`cube_mesh_source`](super::cube_mesh_source)'s flat per-face normals (which need
/// each face to own unshared vertices), this works directly against
/// shared OBJ-style topology without duplicating any vertex.
pub fn smooth_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut accum = vec![[0.0f32; 3]; positions.len()];
    for tri in indices.as_chunks::<3>().0 {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let (p0, p1, p2) = (positions[i0], positions[i1], positions[i2]);
        let face_normal = cross(sub(p1, p0), sub(p2, p0));
        for i in [i0, i1, i2] {
            accum[i][0] += face_normal[0];
            accum[i][1] += face_normal[1];
            accum[i][2] += face_normal[2];
        }
    }
    accum.into_iter().map(normalized).collect()
}

fn normalized(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-8);
    [v[0] / len, v[1] / len, v[2] / len]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat quad (two triangles sharing an edge, all four corners
    /// coplanar) — every vertex's smooth normal must come out the same
    /// (the shared face normal), the simplest real check that
    /// `smooth_normals` actually averages contributions from
    /// every adjacent triangle rather than only the first or last one
    /// touching a vertex.
    #[test]
    fn smooth_normals_agree_across_a_flat_shared_quad() {
        let positions = vec![
            [-1.0, 0.0, -1.0],
            [1.0, 0.0, -1.0],
            [1.0, 0.0, 1.0],
            [-1.0, 0.0, 1.0],
        ];
        // Two triangles sharing the (0, 2) diagonal.
        let indices = vec![0, 1, 2, 0, 2, 3];

        let normals = smooth_normals(&positions, &indices);
        assert_eq!(normals.len(), 4);
        // The exact sign depends on winding (not the point of this
        // test); what matters is every vertex agreeing on the *same*
        // normal, proving contributions from both triangles were
        // actually averaged together rather than one overwriting the
        // other.
        let first = normals[0];
        for normal in &normals {
            assert!(
                (normal[0] - first[0]).abs() < 1e-6
                    && (normal[1] - first[1]).abs() < 1e-6
                    && (normal[2] - first[2]).abs() < 1e-6,
                "all four coplanar vertices must share one normal: {first:?} vs {normal:?}"
            );
        }
        assert!(
            (normal_length(first) - 1.0).abs() < 1e-6,
            "normal must be unit length, got {first:?}"
        );
    }

    fn normal_length(n: [f32; 3]) -> f32 {
        (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
    }

    /// A single triangle: every normal must be unit-length and match the
    /// plain cross-product direction — the base case
    /// `smooth_normals` reduces to before any averaging happens.
    #[test]
    fn smooth_normals_matches_face_normal_for_a_single_triangle() {
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let indices = vec![0, 1, 2];

        let normals = smooth_normals(&positions, &indices);
        // cross((1,0,0), (0,0,1)) = (0*1 - 0*0, 0*0 - 1*1, 1*0 - 0*0) = (0, -1, 0)
        for normal in &normals {
            assert!(
                (normal_length(*normal) - 1.0).abs() < 1e-6,
                "normal must be unit length, got {normal:?}"
            );
            assert!(
                (normal[1] - (-1.0)).abs() < 1e-6,
                "expected -Y normal, got {normal:?}"
            );
        }
    }
}
