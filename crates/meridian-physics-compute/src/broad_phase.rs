//! Batch [`BroadPhase::find_candidate_pairs`](meridian_physics_core::generic::BroadPhase::find_candidate_pairs)'s
//! AABB overlap sweep through `compute-runtime` — the last of the three
//! "aren't batched yet" items [`crate::narrow_phase`]'s own module doc
//! left open (`Integrator`/`NarrowPhase::test_pair`/`generate_contacts`
//! are covered by [`crate::rigid_body`]/[`crate::narrow_phase`]; this
//! module and [`crate::constraint_solver`] close out the remaining two).
//!
//! **Reuses [`RigidBody::aabb`] and [`gac_core::generic::Aabb::overlaps`]
//! directly — does not reimplement the bounding-box math.** Per
//! CLAUDE.md's "don't drag another crate's logic into your own" rule,
//! bounding-box geometry belongs to `gac-core`, and which AABB a body
//! has belongs to `physics-core`; this module's only job is running the
//! same pairwise overlap test many candidate pairs at once instead of a
//! nested loop.
//!
//! Generic over `F: GaFlavor`, like every other kernel in this crate
//! that isn't a WGSL kernel: an AABB overlap test is the same sequence
//! of comparisons regardless of scalar flavor, no GPU-dispatch
//! constraint forces a float/fixed split.
//!
//! **Fixed 1-bool-per-candidate-pair output shape**, the same "no
//! variable-output-per-thread problem" reasoning
//! [`crate::narrow_phase::NarrowPhaseTestPairKernel`] uses: every
//! `(i, j)` pair with `i < j` gets exactly one `bool` slot (overlaps or
//! not), computed in parallel; [`BroadPhasePairsKernel::pairs`] then
//! filters that fixed-size array down to the surviving pairs, in the
//! same ascending `(i, j)` order `BroadPhase::find_candidate_pairs`'s
//! own nested loop produces — so this is a drop-in replacement for it,
//! not just numerically similar.

use std::sync::Mutex;

use meridian_compute_runtime::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac_core::generic::GaFlavor;
use meridian_physics_core::generic::RigidBody;

/// Batch-tests every candidate pair `(i, j)`, `i < j`, among `bodies`
/// for AABB overlap — one dispatch instead of `BroadPhase`'s O(n²)
/// nested loop. [`pairs`](Self::pairs) returns the surviving pairs in
/// the same ascending order `BroadPhase::find_candidate_pairs` itself
/// produces.
#[derive(Debug)]
pub struct BroadPhasePairsKernel<F: GaFlavor>
where
    F: Sync,
    F::Scalar: Send + Sync,
    F::Vector: Send + Sync,
    F::Bivector: Send + Sync,
    F::Rotor: Send + Sync,
    F::Motor: Send + Sync,
{
    pub bodies: Vec<RigidBody<F>>,
    /// Every `(i, j)`, `i < j` pair among `bodies` — precomputed once at
    /// construction (cheap: just index generation, no overlap test) so
    /// `dispatch` only needs a flat `parallel_for` over this list rather
    /// than deriving a triangular-number index mapping on every call.
    candidate_pairs: Vec<(usize, usize)>,
    overlaps: Mutex<Vec<bool>>,
}

impl<F: GaFlavor> BroadPhasePairsKernel<F>
where
    F: Sync,
    F::Scalar: Send + Sync,
    F::Vector: Send + Sync,
    F::Bivector: Send + Sync,
    F::Rotor: Send + Sync,
    F::Motor: Send + Sync,
{
    pub fn new(bodies: Vec<RigidBody<F>>) -> Self {
        let n = bodies.len();
        let mut candidate_pairs = Vec::with_capacity(n.saturating_mul(n) / 2);
        for i in 0..n {
            for j in (i + 1)..n {
                candidate_pairs.push((i, j));
            }
        }
        let overlaps = Mutex::new(vec![false; candidate_pairs.len()]);
        Self {
            bodies,
            candidate_pairs,
            overlaps,
        }
    }

    /// The surviving candidate pairs from the most recent
    /// [`ComputeKernel::dispatch`] call, in the same ascending `(i, j)`
    /// order `BroadPhase::find_candidate_pairs` produces. Empty if
    /// `dispatch` hasn't run yet.
    pub fn pairs(&self) -> Vec<(usize, usize)> {
        let overlaps = self.overlaps.lock().unwrap();
        self.candidate_pairs
            .iter()
            .zip(overlaps.iter())
            .filter(|(_, &overlaps)| overlaps)
            .map(|(&pair, _)| pair)
            .collect()
    }
}

impl<F: GaFlavor> ComputeKernel for BroadPhasePairsKernel<F>
where
    F: Sync,
    F::Scalar: Send + Sync,
    F::Vector: Send + Sync,
    F::Bivector: Send + Sync,
    F::Rotor: Send + Sync,
    F::Motor: Send + Sync,
{
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.candidate_pairs.len());
        context.parallel_for(count, |k| {
            let (i, j) = self.candidate_pairs[k];
            let overlap = self.bodies[i].aabb().overlaps(&self.bodies[j].aabb());
            self.overlaps.lock().unwrap()[k] = overlap;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_compute_runtime::DispatchSize;
    use meridian_gac_core::float_ga::FloatFlavor;
    use meridian_gac_core::{Motor3, Vec3};
    use meridian_physics_core::generic::{BroadPhase, ColliderShape};

    fn sphere_at(x: f32) -> RigidBody<FloatFlavor> {
        RigidBody {
            frame: Motor3::translation(Vec3::new(x, 0.0, 0.0)),
            velocity: Vec3::ZERO,
            angular_velocity: meridian_gac_core::Bivector3::ZERO,
            mass: 1.0,
            shape: ColliderShape::Sphere { radius: 0.5 },
        }
    }

    /// The kernel's batched pairs, run through `compute-runtime`'s CPU
    /// `parallel_for`, must be bit-for-bit identical (same pairs, same
    /// order) to calling `BroadPhase::find_candidate_pairs` directly —
    /// this is a drop-in replacement, not an approximation.
    #[test]
    fn dispatch_matches_direct_find_candidate_pairs() {
        let bodies = vec![
            sphere_at(0.0),
            sphere_at(0.6),  // overlaps 0
            sphere_at(10.0), // isolated
            sphere_at(10.4), // overlaps 2, not 0/1
        ];

        let mut broad_phase = BroadPhase::<FloatFlavor>::new();
        let expected = broad_phase.find_candidate_pairs(&bodies).to_vec();

        let context = ComputeContext::new();
        let kernel = BroadPhasePairsKernel::new(bodies);
        kernel.dispatch(&context, DispatchSize::linear(6));

        assert_eq!(kernel.pairs(), expected);
    }

    /// Exercises the CPU `parallel_for` backend's actual parallel path
    /// (above `ComputeContext`'s default 1024-item threshold) with a
    /// real, non-trivial overlap pattern (a chain of touching spheres).
    #[test]
    fn large_batch_matches_direct_find_candidate_pairs() {
        // 60 spheres, radius 0.5, spaced 0.4 apart -> every consecutive
        // pair overlaps, none else do -> 60*59/2 = 1770 candidate pairs,
        // comfortably above the parallel threshold.
        let bodies: Vec<_> = (0..60).map(|i| sphere_at(i as f32 * 0.4)).collect();

        let mut broad_phase = BroadPhase::<FloatFlavor>::new();
        let expected = broad_phase.find_candidate_pairs(&bodies).to_vec();

        let context = ComputeContext::new();
        let kernel = BroadPhasePairsKernel::new(bodies);
        kernel.dispatch(&context, DispatchSize::linear(1770));

        assert_eq!(kernel.pairs(), expected);
    }
}
