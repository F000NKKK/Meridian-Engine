//! Plane.

use super::Shape;
use crate::flavor::{GaFlavor, ScalarLike, VectorLike};

/// A half-space `normal . p + d >= 0` — a point satisfying this is on the
/// plane's "inside". Another plain geometric primitive with no domain
/// meaning of its own (`rendering-domain`'s `Frustum`/[`ConvexVolume`](crate::ConvexVolume) is a
/// handful of these; a future physics ground plane or clipping pass would
/// be another consumer), so it lives here rather than being redefined per
/// subsystem — see [`Aabb`](crate::Aabb)'s doc comment for the same reasoning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane<F: GaFlavor> {
    pub normal: F::Vector,
    pub d: F::Scalar,
}

impl<F: GaFlavor> Plane<F> {
    /// Rescales `normal`/`d` so `normal` is unit length, without changing
    /// which half-space the plane represents. Needed before
    /// [`distance`](Self::distance) is a true Euclidean distance rather
    /// than just a correctly-signed value.
    pub fn normalize(self) -> Self {
        let len = self.normal.length();
        Plane {
            normal: F::Vector::new(
                self.normal.x() / len,
                self.normal.y() / len,
                self.normal.z() / len,
            ),
            d: self.d / len,
        }
    }

    /// Signed distance from `p` to the plane: positive on the "inside"
    /// half-space, negative on the other side.
    pub fn distance(&self, p: F::Vector) -> F::Scalar {
        self.normal.dot(p) + self.d
    }

    /// Whether `shape` is at least partially on this plane's "inside"
    /// half-space: `false` only when `shape` is entirely on the outside.
    /// A shape is fully outside iff *every* point of it has negative
    /// distance, i.e. iff even its best-case point — the one farthest
    /// *along* the normal, `shape.support(normal)` — is still outside; if
    /// that best-case point is inside, the shape can't be fully excluded.
    /// Generic over any [`Shape`], not just [`Aabb`](crate::Aabb) — this is what makes
    /// [`ConvexVolume::intersects`](crate::ConvexVolume::intersects) work for every shape without the
    /// plane needing to know what kind of shape it's testing.
    pub fn contains<S: Shape<F>>(&self, shape: &S) -> bool {
        self.distance(shape.support(self.normal)) >= F::Scalar::ZERO
    }
}
