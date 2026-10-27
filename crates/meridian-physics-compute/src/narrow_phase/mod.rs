//! Batch [`NarrowPhase::test_pair`](meridian_physics_core::collision::NarrowPhase)
//! across many candidate pairs through `compute-runtime` — the
//! `NarrowPhase` half of the rigid-body-pipeline batching this crate's
//! own module doc describes (see [`crate::broad_phase`] for the broad
//! phase half, [`crate::constraint_solver`] for the solver half).
//! Deliberately targets `test_pair`, not
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
use meridian_gac_core::GaFlavor;
use meridian_physics_core::collision::{Contact, NarrowPhase};
use meridian_physics_core::dynamics::RigidBody;

/// Batch-tests `pairs` (indices into `bodies`, typically
/// [`BroadPhase::find_candidate_pairs`](meridian_physics_core::collision::BroadPhase::find_candidate_pairs)'s
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
        self.results
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
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
            self.results
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)[i] = contact;
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
/// [`ConstraintSolver::resolve`](meridian_physics_core::dynamics::ConstraintSolver::resolve)'s
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
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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
            for (slot, contact) in slots.iter_mut().zip(contacts) {
                *slot = Some(contact);
            }
            self.results
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)[i] = slots;
        });
    }
}

#[cfg(test)]
mod tests;
