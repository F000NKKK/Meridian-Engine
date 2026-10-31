//! The support-function interface every convex shape implements.

use crate::flavor::GaFlavor;

/// A convex shape describable by its support function: the point of the
/// shape farthest along a given direction. This is the standard
/// convex-geometry interface (the same one GJK/EPA-style algorithms are
/// built on) that lets a plane or [`ConvexVolume`](crate::ConvexVolume) test *any* shape
/// without a hardcoded list of shape variants, and lets a new shape (a
/// capsule, a convex hull) work with every existing plane/volume test for
/// free just by implementing this one method — no double-dispatch, no
/// shape x shape combinatorial explosion.
pub trait Shape<F: GaFlavor> {
    /// Returns the shape's farthest point along `direction`.
    fn support(&self, direction: F::Vector) -> F::Vector;
}

// A blanket `impl<F: GaFlavor> Shape<F> for F::Vector` (treating a point
// as a degenerate shape) is deliberately *not* here: Rust's coherence
// checker can't prove `F::Vector` is disjoint from `Aabb<F>`/`Sphere<F>`/
// etc. for every possible future `GaFlavor` impl, so it conflicts with
// the `Shape<F>` impls below. each flavor module provide their
// own concrete `impl Shape<FloatFlavor> for Vec3`/
// `impl Shape<FixedFlavor> for FixedVec3` instead — the same
// "a point is a degenerate shape" behavior, just per-flavor.
