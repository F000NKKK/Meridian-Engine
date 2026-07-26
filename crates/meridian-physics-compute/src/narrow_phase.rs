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

/// Per-pair cap on [`GenerateContactsKernel`]'s fixed-size output slot —
/// the "per-pair count prefix-sum into a flattened output buffer"
/// technique [`crate`]'s and docs/roadmap.md's own notes about batching
/// `generate_contacts` describe, specialized to a fixed cap instead of a
/// true dynamic prefix-sum, since `physics-core::generic::face_manifold`
/// itself already hard-caps a box-box manifold at 4 points (`.take(4)`)
/// — the same fixed-capacity-array approach `graphics-core::submission`'s
/// `MAX_LIGHTS` uses for its own per-frame uniform array. A pair
/// producing more than this many contacts (impossible with today's
/// `face_manifold`, but not guaranteed forever) is truncated with a
/// `meridian_foundation::log_warn!`, not silently dropped — matching
/// `MAX_LIGHTS`'s own policy.
pub const MAX_CONTACTS_PER_PAIR: usize = 4;

/// Batch-expands `pairs` into full contact manifolds via
/// [`NarrowPhase::generate_contacts`] — the manifold-aware counterpart to
/// [`NarrowPhaseTestPairKernel`], for callers that need
/// [`ConstraintSolver::resolve`](meridian_physics_core::generic::ConstraintSolver::resolve)'s
/// real multi-point input (see this module's own doc comment for why
/// `generate_contacts`'s variable per-pair output needed a different
/// kernel shape than `test_pair`'s fixed 1:1 one).
///
/// **Reuses `NarrowPhase::generate_contacts` itself, called with a
/// one-pair slice per dispatched item — it does not reimplement SAT or
/// face-clipping.** Per CLAUDE.md's "don't drag another crate's logic
/// into your own" rule, contact-manifold geometry belongs to
/// `physics-core`; this kernel's only job is *how many threads run it
/// and in what shape the output lands*, not the geometry itself. A
/// single-pair `generate_contacts` call produces the exact same
/// `Contact`s (same order, same values) that pair would contribute
/// inside a whole-batch call, since the algorithm has no cross-pair
/// state — so [`GenerateContactsKernel::results`], flattened in pair
/// order, is provably identical to calling `generate_contacts(bodies,
/// pairs)` once directly (see this module's tests).
#[derive(Debug)]
pub struct GenerateContactsKernel<F: GaFlavor>
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
    results: Mutex<Vec<[Option<Contact<F>>; MAX_CONTACTS_PER_PAIR]>>,
}

impl<F: GaFlavor> GenerateContactsKernel<F>
where
    F: Sync,
    F::Scalar: Send + Sync,
    F::Vector: Send + Sync,
    F::Bivector: Send + Sync,
    F::Rotor: Send + Sync,
    F::Motor: Send + Sync,
{
    pub fn new(bodies: Vec<RigidBody<F>>, pairs: Vec<(usize, usize)>) -> Self {
        let results = Mutex::new(vec![[None; MAX_CONTACTS_PER_PAIR]; pairs.len()]);
        Self {
            narrow_phase: NarrowPhase::new(),
            bodies,
            pairs,
            results,
        }
    }

    /// This kernel's contacts from the most recent [`ComputeKernel::dispatch`]
    /// call, flattened back into pair order — the same shape and order
    /// `NarrowPhase::generate_contacts(bodies, pairs)` itself returns
    /// (see this type's own doc comment for why that equivalence holds).
    /// Empty if `dispatch` hasn't run yet.
    pub fn results(&self) -> Vec<Contact<F>> {
        self.results
            .lock()
            .unwrap()
            .iter()
            .flat_map(|slots| slots.iter().filter_map(|slot| *slot))
            .collect()
    }
}

impl<F: GaFlavor> ComputeKernel for GenerateContactsKernel<F>
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
            let contacts = self
                .narrow_phase
                .generate_contacts(&self.bodies, core::slice::from_ref(&self.pairs[i]));
            let mut slots = [None; MAX_CONTACTS_PER_PAIR];
            if contacts.len() > MAX_CONTACTS_PER_PAIR {
                meridian_foundation::log_warn!(
                    "GenerateContactsKernel: pair {:?} produced {} contacts, \
                     exceeding MAX_CONTACTS_PER_PAIR ({}) — truncating",
                    self.pairs[i],
                    contacts.len(),
                    MAX_CONTACTS_PER_PAIR
                );
            }
            for (slot, contact) in slots.iter_mut().zip(contacts.into_iter()) {
                *slot = Some(contact);
            }
            self.results.lock().unwrap()[i] = slots;
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

    fn cuboid_at(x: f32, y: f32, half: f32) -> RigidBody<FloatFlavor> {
        RigidBody {
            frame: Motor3::translation(Vec3::new(x, y, 0.0)),
            velocity: Vec3::ZERO,
            angular_velocity: meridian_gac_core::Bivector3::ZERO,
            mass: 1.0,
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(half, half, half),
            },
        }
    }

    /// A wide, flat floor and a small box resting flush on top — the
    /// same shape `physics-core::float`'s own
    /// `full_step_cuboid_settles_on_static_cuboid_floor_without_sinking_through`
    /// test uses, chosen specifically because the box's face sits well
    /// inside the floor's much larger face, guaranteeing `face_manifold`
    /// keeps all 4 corners (a real multi-point manifold, not a
    /// degenerate edge/corner contact).
    fn floor_and_box() -> Vec<RigidBody<FloatFlavor>> {
        vec![
            RigidBody {
                shape: ColliderShape::Cuboid {
                    half_extents: Vec3::new(10.0, 5.0, 10.0),
                },
                ..cuboid_at(0.0, -5.0, 0.0)
            },
            RigidBody {
                shape: ColliderShape::Cuboid {
                    half_extents: Vec3::new(0.5, 0.5, 0.5),
                },
                ..cuboid_at(0.0, 0.45, 0.0)
            },
        ]
    }

    /// `GenerateContactsKernel`'s batched (one-dispatch-per-pair) output,
    /// flattened back in pair order, must be identical to calling
    /// `NarrowPhase::generate_contacts` once directly on the whole
    /// `pairs` slice — including the multi-point box-box manifold case
    /// this kernel exists for (unlike `NarrowPhaseTestPairKernel`, which
    /// only ever sees `test_pair`'s single-point collapse).
    #[test]
    fn dispatch_matches_direct_generate_contacts() {
        let bodies = floor_and_box();
        let pairs = vec![(0usize, 1usize)];

        let narrow_phase = NarrowPhase::<FloatFlavor>::new();
        let expected = narrow_phase.generate_contacts(&bodies, &pairs);
        assert!(
            expected.len() > 1,
            "test setup should produce a multi-point manifold, got {}",
            expected.len()
        );

        let context = ComputeContext::new();
        let kernel = GenerateContactsKernel::new(bodies, pairs);
        kernel.dispatch(&context, DispatchSize::linear(kernel.pairs.len() as u32));

        let got = kernel.results();
        assert_eq!(got.len(), expected.len());
        for (g, e) in got.iter().zip(expected.iter()) {
            assert_eq!(g.a, e.a);
            assert_eq!(g.b, e.b);
            assert_eq!(g.normal, e.normal);
            assert_eq!(g.penetration, e.penetration);
            assert_eq!(g.point, e.point);
            assert_eq!(g.suppress_angular_response, e.suppress_angular_response);
        }
    }

    /// A mix of overlapping and non-overlapping pairs, and a
    /// non-cuboid-cuboid pair (single-point `test_pair` fallback inside
    /// `generate_contacts` itself) alongside a multi-point one — the
    /// kernel must reproduce both shapes correctly in the same batch.
    #[test]
    fn dispatch_matches_direct_generate_contacts_mixed_pairs() {
        let mut bodies = floor_and_box(); // 0: floor, 1: box resting on floor (multi-point)
        bodies.push(sphere_at(0.6)); // 2: elsewhere, doesn't overlap 0 or 1
        bodies.push(cuboid_at(100.0, 0.0, 1.0)); // 3: far away, no overlap with 0
        let pairs = vec![(0usize, 1usize), (0, 3), (1, 2)];

        let narrow_phase = NarrowPhase::<FloatFlavor>::new();
        let expected = narrow_phase.generate_contacts(&bodies, &pairs);

        let context = ComputeContext::new();
        let kernel = GenerateContactsKernel::new(bodies, pairs);
        kernel.dispatch(&context, DispatchSize::linear(kernel.pairs.len() as u32));

        let got = kernel.results();
        assert_eq!(got.len(), expected.len());
        for (g, e) in got.iter().zip(expected.iter()) {
            assert_eq!(g.a, e.a);
            assert_eq!(g.b, e.b);
            assert_eq!(g.point, e.point);
            assert_eq!(g.penetration, e.penetration);
        }
    }
}
