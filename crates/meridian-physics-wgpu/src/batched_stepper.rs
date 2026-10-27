//! The batched physics step on compute-runtime kernels.

use meridian_compute_app::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_physics_app::{PhysicsStepper, PhysicsSubsystem, RELAXATION_ITERATIONS};

use crate::broad_phase::BroadPhasePairsKernel;
use crate::constraint_solver::ConstraintSolverBatchKernel;
use crate::narrow_phase::GenerateContactsKernel;
use crate::rigid_body::RigidBodyIntegratorKernel;

/// Steps physics through this crate's batched kernels (integrator, broad
/// phase, narrow phase, constraint solver) dispatched on a
/// [`ComputeContext`], the `PhysicsStepper` counterpart of
/// `SequentialStepper`.
///
/// **Same algorithm, different dispatch shape.** It mirrors the
/// sequential step (integrate, `RELAXATION_ITERATIONS` velocity-only
/// passes, one positional-correction pass) with one deliberate
/// difference: broad and narrow phase run *once* per tick, not once per
/// relaxation pass, since `resolve_velocity` never changes a body's
/// `frame` and the contact set genuinely cannot change between
/// velocity-only passes (see `ConstraintSolverBatchKernel`).
///
/// "Batched", not yet literally GPU-executing: every kernel here goes
/// through `ComputeContext::parallel_for`; only the broad phase has a
/// WGSL variant (`broad_phase::float::BroadPhaseGpuKernel`), not wired in.
#[derive(Debug)]
pub struct BatchedStepper {
    context: ComputeContext,
}

impl BatchedStepper {
    /// A stepper dispatching on `context`, which the composition root builds
    /// from its compute adapters.
    #[must_use]
    pub fn new(context: ComputeContext) -> Self {
        Self { context }
    }
}

impl PhysicsStepper for BatchedStepper {
    fn step(&mut self, physics: &mut PhysicsSubsystem, dt: f32) {
        let body_count = physics.bodies.len();

        let integrator_kernel =
            RigidBodyIntegratorKernel::new(physics.integrator, physics.bodies.clone(), dt);
        integrator_kernel.dispatch(&self.context, DispatchSize::linear(body_count as u32));
        let integrated = integrator_kernel.results();

        // Upper bounds, not exact counts: every kernel clamps its dispatch
        // count to its real candidate/pair list length internally.
        let pair_upper_bound = (body_count.saturating_mul(body_count.max(1))) as u32;
        let broad_kernel = BroadPhasePairsKernel::new(integrated.clone());
        broad_kernel.dispatch(&self.context, DispatchSize::linear(pair_upper_bound));
        let pairs = broad_kernel.pairs();

        let contacts_kernel = GenerateContactsKernel::new(integrated, pairs);
        contacts_kernel.dispatch(&self.context, DispatchSize::linear(pair_upper_bound));
        let contacts = contacts_kernel.results();

        let solver_kernel = ConstraintSolverBatchKernel::new(
            physics.solver,
            contacts_kernel.bodies.clone(),
            contacts,
        );
        solver_kernel.step(&self.context, RELAXATION_ITERATIONS);

        physics.bodies = solver_kernel.bodies();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use meridian_compute_app::testing::SequentialDevice;
    use meridian_physics_app::testing::run_stepper_contract;

    use super::*;

    #[test]
    fn batched_stepper_passes_the_stepper_contract() {
        let context = ComputeContext::new(Arc::new(SequentialDevice));
        run_stepper_contract(&mut BatchedStepper::new(context));
    }
}
