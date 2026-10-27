//! A single mesh face.

/// A single mesh face: the ordered vertex indices of a polygon (not
/// necessarily a triangle — arbitrary point count) into some [`Mesh`](crate::Mesh)'s
/// `vertices`. Scalar-flavor-independent (indices are plain `usize`), so
/// this lives once, not once per [`GaFlavor`](crate::GaFlavor) the way [`Mesh`](crate::Mesh) itself
/// does (it holds flavor-typed vertex positions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub indices: Vec<usize>,
}

impl Face {
    pub fn triangle(a: usize, b: usize, c: usize) -> Self {
        Self {
            indices: vec![a, b, c],
        }
    }

    /// Fan-triangulates this face: `(indices[0], indices[i], indices[i + 1])`
    /// for every `i` in `1..indices.len() - 1`. Correct for convex faces
    /// — every triangle and quad this workspace's own mesh producers
    /// generate is convex; a non-convex n-gon would need real
    /// ear-clipping, not implemented since nothing here produces one.
    pub fn triangles(&self) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        let first = self.indices[0];
        self.indices[1..]
            .windows(2)
            .map(move |w| (first, w[0], w[1]))
    }
}
