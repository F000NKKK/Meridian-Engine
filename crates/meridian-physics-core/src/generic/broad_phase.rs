//! Broad phase: candidate pairs from overlapping bounds.

use meridian_gac_core::generic::{Aabb, GaFlavor, Shape, VectorLike};

use super::{ColliderShape, RigidBody, axis_x, axis_y, axis_z};

/// Computes `body`'s world-space AABB — a free function (rather than a
/// method) purely because it's called through `.map(aabb_of)` below;
/// [`RigidBody::aabb`](super::RigidBody::aabb) is the public entry point, defined right after
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

/// A broad-phase acceleration structure (BVH, spatial hash, ...). Owned
/// here, not in `physics-driver` — see docs/physics-design.md. Currently a
/// naive O(n²) AABB sweep.
#[derive(Debug)]
pub struct BroadPhase<F: GaFlavor> {
    pairs: Vec<(usize, usize)>,
    _flavor: core::marker::PhantomData<F>,
}

impl<F: GaFlavor> Default for BroadPhase<F> {
    fn default() -> Self {
        Self {
            pairs: Vec::new(),
            _flavor: core::marker::PhantomData,
        }
    }
}

impl<F: GaFlavor> BroadPhase<F> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns index pairs into `bodies` whose AABBs overlap — candidates
    /// for narrow-phase, not confirmed contacts.
    pub fn find_candidate_pairs(&mut self, bodies: &[RigidBody<F>]) -> &[(usize, usize)] {
        self.pairs.clear();
        let aabbs: Vec<Aabb<F>> = bodies.iter().map(aabb_of).collect();
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                if aabbs[i].overlaps(&aabbs[j]) {
                    self.pairs.push((i, j));
                }
            }
        }
        &self.pairs
    }
}
