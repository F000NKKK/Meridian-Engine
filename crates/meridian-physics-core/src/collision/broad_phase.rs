//! Broad phase: candidate pairs from overlapping bounds.

use meridian_gac_core::volume::Aabb;
use meridian_gac_core::{GaFlavor, Shape, VectorLike};

use super::{ColliderShape, axis_x, axis_y, axis_z};
use crate::dynamics::RigidBody;

/// Computes `body`'s world-space AABB — a free function (rather than a
/// method) purely because it's called through `.map(aabb_of)` below;
/// [`RigidBody::aabb`](crate::dynamics::RigidBody::aabb) is the public entry point, defined right after
/// `impl RigidBody` above and forwarding here, so this logic exists in
/// exactly one place regardless of caller (`BroadPhase` itself, or an
/// external adapter crate like `meridian-physics-compute` batching the
/// same broad-phase sweep — see docs/dependency-rules.md and CLAUDE.md's
/// "don't drag another crate's logic into your own" rule).
pub(crate) fn aabb_of<F: GaFlavor>(body: &RigidBody<F>) -> Aabb<F> {
    match body.shape {
        ColliderShape::Sphere { radius } => Aabb::from_sphere(body.position(), radius),
        ColliderShape::Cuboid { half_extents } => {
            // The world-space AABB of an oriented box: query the box's own
            // support function along each world axis — exactly the same
            // `Shape` interface `ConvexVolume`/`Frustum` use, applied here
            // to a different generic problem (bounding an oriented shape),
            // not a bespoke box-AABB formula.
            let obb = body.as_obb(half_extents);
            let (x, y, z) = (axis_x::<F>(), axis_y::<F>(), axis_z::<F>());
            Aabb {
                min: F::Vector::new(
                    obb.support(-x).x(),
                    obb.support(-y).y(),
                    obb.support(-z).z(),
                ),
                max: F::Vector::new(obb.support(x).x(), obb.support(y).y(), obb.support(z).z()),
            }
        }
    }
}

/// Broad phase: sweep-and-prune over the x axis. Bodies are sorted by AABB
/// minimum x, and each body is only tested against the following bodies
/// whose minimum x still lies inside its own x extent, so cost scales with
/// bodies that are near each other along x rather than with all pairs.
/// Needs only ordering and comparison from the scalar, so it behaves
/// identically for the float and the fixed-point flavor. Output is the same
/// set, in the same ascending `(i, j)` order, as an exhaustive O(n²) sweep.
#[derive(Debug)]
pub struct BroadPhase<F: GaFlavor> {
    pairs: Vec<(usize, usize)>,
    aabbs: Vec<Aabb<F>>,
    order: Vec<usize>,
}

impl<F: GaFlavor> Default for BroadPhase<F> {
    fn default() -> Self {
        Self {
            pairs: Vec::new(),
            aabbs: Vec::new(),
            order: Vec::new(),
        }
    }
}

impl<F: GaFlavor> BroadPhase<F> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns index pairs into `bodies` whose AABBs overlap — candidates
    /// for narrow-phase, not confirmed contacts — as `(i, j)` with `i < j`,
    /// sorted ascending.
    pub fn find_candidate_pairs(&mut self, bodies: &[RigidBody<F>]) -> &[(usize, usize)] {
        self.pairs.clear();
        self.aabbs.clear();
        self.aabbs.extend(bodies.iter().map(aabb_of));
        self.order.clear();
        self.order.extend(0..bodies.len());
        let aabbs = &self.aabbs;
        self.order.sort_by(|&a, &b| {
            aabbs[a]
                .min
                .x()
                .partial_cmp(&aabbs[b].min.x())
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        self.sweep();
        self.pairs.sort_unstable();
        &self.pairs
    }

    /// Walks the x-sorted order, testing each body against the later ones
    /// that start before it ends along x.
    fn sweep(&mut self) {
        for (rank, &i) in self.order.iter().enumerate() {
            let reach = self.aabbs[i].max.x();
            for &j in &self.order[rank + 1..] {
                if self.aabbs[j].min.x() > reach {
                    break;
                }
                if self.aabbs[i].overlaps(&self.aabbs[j]) {
                    self.pairs.push((i.min(j), i.max(j)));
                }
            }
        }
    }
}
