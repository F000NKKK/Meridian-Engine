//! Batch [`NarrowPhase::test_pair`](meridian_physics_core::generic::NarrowPhase)
//! across many candidate pairs through `compute-runtime` — the
//! `NarrowPhase` half of the "`BroadPhase`/`NarrowPhase`/
//! `ConstraintSolver` aren't batched yet" follow-up
//! [`crate::rigid_body`] left open. Deliberately targets `test_pair`, not
//! `NarrowPhase::generate_contacts`: `test_pair` is one candidate pair in,
//! one `Option<Contact<F>>` out — the same fixed 1:1 shape
//! [`meridian_gac_compute::MotorComposeKernel`] batches independent
//! `(child, parent)` motor pairs with. `generate_contacts` produces a
//! *variable* number of manifold points per box-box pair (see that
//! method's own doc comment), which doesn't fit a kernel whose per-item
//! output is a fixed-size slot — batching that shape (a per-pair count
//! prefix-sum into a flattened output buffer, the standard GPU technique
//! for variable-output-per-thread) is real further follow-up, not
//! attempted here.
//!
//! Like [`crate::rigid_body::RigidBodyIntegratorKernel`], this is generic
//! over `F: GaFlavor` rather than float/fixed-split: `test_pair`'s exact
//! overlap tests (sphere-sphere distance, sphere-cuboid closest point,
//! cuboid-cuboid SAT) are the same sequence of GA operations regardless
//! of scalar flavor, no GPU-dispatch constraint forces a split, and CPU
//! dispatch through [`meridian_compute_runtime::ComputeContext::parallel_for`]
//! is the real, useful step here — the same reasoning
//! `RigidBodyIntegratorKernel`'s own doc comment gives.

use std::sync::Mutex;

use meridian_compute_runtime::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac_core::generic::GaFlavor;
use meridian_physics_core::generic::{Contact, NarrowPhase, RigidBody};

/// Batch-tests `pairs` (indices into `bodies`, typically
/// [`BroadPhase::find_candidate_pairs`](meridian_physics_core::generic::BroadPhase::find_candidate_pairs)'s
/// output) for exact overlap, one dispatch instead of a loop.
/// [`NarrowPhaseTestPairKernel::results`] holds one `Option<Contact<F>>`
/// per pair, in the same order as `pairs` — `None` where `test_pair`
/// itself would return `None` (not overlapping).
#[derive(Debug)]
pub struct NarrowPhaseTestPairKernel<F: GaFlavor>
where
    F: Sync,
    F::Scalar: Send + Sync,
    F::Vector: Send + Sync,
    F::Bivector: Send + Sync,
    F::Rotor: Send + Sync,
    F::Motor: Send + Sync,
{
    narrow_phase: NarrowPhase<F>,
    pub bodies: Vec<RigidBody<F>>,
    pub pairs: Vec<(usize, usize)>,
    results: Mutex<Vec<Option<Contact<F>>>>,
}

impl<F: GaFlavor> NarrowPhaseTestPairKernel<F>
where
    F: Sync,
    F::Scalar: Send + Sync,
    F::Vector: Send + Sync,
    F::Bivector: Send + Sync,
    F::Rotor: Send + Sync,
    F::Motor: Send + Sync,
{
    pub fn new(bodies: Vec<RigidBody<F>>, pairs: Vec<(usize, usize)>) -> Self {
        let results = Mutex::new(vec![None; pairs.len()]);
        Self {
            narrow_phase: NarrowPhase::new(),
            bodies,
            pairs,
            results,
        }
    }

    /// This kernel's `Option<Contact<F>>` per pair from the most recent
    /// [`ComputeKernel::dispatch`] call — all `None` if `dispatch` hasn't
    /// run yet.
    pub fn results(&self) -> Vec<Option<Contact<F>>> {
        self.results.lock().unwrap().clone()
    }
}

impl<F: GaFlavor> ComputeKernel for NarrowPhaseTestPairKernel<F>
where
    F: Sync,
    F::Scalar: Send + Sync,
    F::Vector: Send + Sync,
    F::Bivector: Send + Sync,
    F::Rotor: Send + Sync,
    F::Motor: Send + Sync,
{
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.pairs.len());
        context.parallel_for(count, |i| {
            let (a, b) = self.pairs[i];
            let contact = self.narrow_phase.test_pair(&self.bodies, a, b);
            self.results.lock().unwrap()[i] = contact;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::float_ga::FloatFlavor;
    use meridian_gac_core::{Motor3, Vec3};
    use meridian_physics_core::generic::ColliderShape;

    fn sphere_at(x: f32) -> RigidBody<FloatFlavor> {
        RigidBody {
            frame: Motor3::translation(Vec3::new(x, 0.0, 0.0)),
            velocity: Vec3::ZERO,
            angular_velocity: meridian_gac_core::Bivector3::ZERO,
            mass: 1.0,
            shape: ColliderShape::Sphere { radius: 0.5 },
        }
    }

    /// The kernel's batched `test_pair` results, run through
    /// `compute-runtime`'s CPU `parallel_for`, must match calling
    /// `NarrowPhase::test_pair` directly for every pair — including
    /// which pairs come back `None` (not overlapping), not just the
    /// overlapping ones.
    #[test]
    fn dispatch_matches_direct_test_pair() {
        // 0 and 1 overlap (spheres of radius 0.5 centered 0.6 apart); 0
        // and 2 don't (centered 10 apart).
        let bodies = vec![sphere_at(0.0), sphere_at(0.6), sphere_at(10.0)];
        let pairs = vec![(0usize, 1usize), (0, 2), (1, 2)];

        let narrow_phase = NarrowPhase::<FloatFlavor>::new();
        let expected: Vec<_> = pairs
            .iter()
            .map(|&(a, b)| narrow_phase.test_pair(&bodies, a, b))
            .collect();

        let context = ComputeContext::new();
        let kernel = NarrowPhaseTestPairKernel::new(bodies, pairs.clone());
        kernel.dispatch(&context, DispatchSize::linear(pairs.len() as u32));

        let got = kernel.results();
        assert_eq!(got.len(), expected.len());
        assert!(got[0].is_some(), "0-1 should overlap");
        assert!(got[1].is_none(), "0-2 should not overlap");
        assert!(got[2].is_none(), "1-2 should not overlap");
        for (g, e) in got.iter().zip(expected.iter()) {
            match (g, e) {
                (Some(gc), Some(ec)) => {
                    assert_eq!(gc.a, ec.a);
                    assert_eq!(gc.b, ec.b);
                    assert_eq!(gc.normal, ec.normal);
                    assert_eq!(gc.penetration, ec.penetration);
                    assert_eq!(gc.point, ec.point);
                }
                (None, None) => {}
                _ => panic!("mismatch: got {g:?}, expected {e:?}"),
            }
        }
    }

    /// Exercises the CPU `parallel_for` backend's actual parallel path
    /// (above `ComputeContext`'s default 1024-item threshold).
    #[test]
    fn large_batch_matches_direct_test_pair() {
        // A chain of spheres 0.4 apart (radius 0.5, so every consecutive
        // pair overlaps) — enough candidate pairs to cross the parallel
        // threshold, with a real, non-trivial overlap pattern.
        let bodies: Vec<_> = (0..1500).map(|i| sphere_at(i as f32 * 0.4)).collect();
        let pairs: Vec<_> = (0..bodies.len() - 1).map(|i| (i, i + 1)).collect();

        let narrow_phase = NarrowPhase::<FloatFlavor>::new();
        let expected: Vec<_> = pairs
            .iter()
            .map(|&(a, b)| narrow_phase.test_pair(&bodies, a, b))
            .collect();

        let context = ComputeContext::new();
        let kernel = NarrowPhaseTestPairKernel::new(bodies, pairs.clone());
        kernel.dispatch(&context, DispatchSize::linear(pairs.len() as u32));

        let got = kernel.results();
        for (g, e) in got.iter().zip(expected.iter()) {
            assert_eq!(g.is_some(), e.is_some());
            if let (Some(gc), Some(ec)) = (g, e) {
                assert_eq!(gc.point, ec.point);
                assert_eq!(gc.penetration, ec.penetration);
            }
        }
    }
}
