//! The stage that steps physics.

use crate::PhysicsDispatchMode;
use crate::RELAXATION_ITERATIONS;
use crate::Stage;
use crate::StageContext;
use meridian_compute_runtime::ComputeKernel;
use meridian_compute_runtime::{ComputeContext, DispatchSize};
use meridian_physics_compute::broad_phase::BroadPhasePairsKernel;
use meridian_physics_compute::constraint_solver::ConstraintSolverBatchKernel;
use meridian_physics_compute::narrow_phase::GenerateContactsKernel;
use meridian_physics_compute::rigid_body::RigidBodyIntegratorKernel;

/// Built-in convenience stage wrapping physics stepping — either
/// [`PhysicsSubsystem::step`](crate::PhysicsSubsystem::step)'s plain sequential loop, or
/// `meridian-physics-compute`'s batched kernel pipeline, selected via
/// [`PhysicsDispatchMode`](crate::PhysicsDispatchMode) rather than two separate types. Most
/// applications that want physics in their runtime can register this
/// directly instead of writing their own [`Stage`](crate::Stage) impl for it.
///
/// **Same algorithm, different dispatch shape — not a semantic
/// rewrite.** [`PhysicsDispatchMode::Batched`](crate::PhysicsDispatchMode::Batched) mirrors
/// `PhysicsSubsystem::step`'s exact structure (integrate,
/// `RELAXATION_ITERATIONS` velocity-only passes, one final
/// positional-correction pass) with one deliberate difference already
/// proven equivalent by `physics-compute`'s own tests: broad/narrow
/// phase run *once* per tick, not once per relaxation pass (see
/// `ConstraintSolverBatchKernel`'s own module doc for why that's
/// harmless — `resolve_velocity` never changes a body's `frame`, only
/// its velocity, so the contact set genuinely can't change between
/// velocity-only passes; `PhysicsSubsystem::step`'s repeated
/// recomputation there is wasteful, not more correct).
///
/// **"Batched," not (yet) literally GPU-executing.** Every kernel in
/// `PhysicsDispatchMode::Batched` dispatches through
/// `ComputeContext::parallel_for`'s CPU backend today — none of the four
/// (integrator/broad-phase/narrow-phase/solver) has a WGSL shader behind
/// it the way `meridian-physics-compute`'s own `float`/`fixed` soft-body
/// kernels do. A real GPU broad-phase kernel exists in
/// `meridian-physics-compute` (`float::BroadPhaseGpuKernel`) as the
/// first concrete step down that path — not yet wired in here, since
/// the other three kernels (integrator's rotor exponential/motor
/// composition, narrow-phase's SAT/manifold clipping, the solver's
/// graph-colored Gauss-Seidel passes) would need their own WGSL first
/// for a `Gpu` dispatch mode to mean "the whole pipeline runs on the
/// GPU" rather than silently mixing GPU and CPU work under one name.
#[derive(Debug)]
pub struct PhysicsStepStage {
    pub dt: f32,
    pub mode: PhysicsDispatchMode,
    context: ComputeContext,
}

impl PhysicsStepStage {
    pub fn new(dt: f32, mode: PhysicsDispatchMode) -> Self {
        Self {
            dt,
            mode,
            context: ComputeContext::new(),
        }
    }

    fn run_batched(&self, ctx: &StageContext) {
        let mut physics = ctx.physics();
        let body_count = physics.bodies.len();

        let integrator_kernel =
            RigidBodyIntegratorKernel::new(physics.integrator, physics.bodies.clone(), self.dt);
        integrator_kernel.dispatch(&self.context, DispatchSize::linear(body_count as u32));
        let integrated = integrator_kernel.results();

        // Upper bounds, not exact counts — every kernel here clamps its
        // own dispatch count to its real candidate/pair list length
        // internally (see each kernel's own `dispatch` — `size.total()
        // .min(self.\u{2026}.len())`), so an over-generous `DispatchSize` is
        // always safe, just occasionally over-allocates a `parallel_for`
        // call that immediately no-ops past the real count.
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

impl Stage for PhysicsStepStage {
    fn run(&mut self, ctx: &StageContext) {
        match self.mode {
            PhysicsDispatchMode::Sequential => ctx.physics().step(self.dt),
            PhysicsDispatchMode::Batched => self.run_batched(ctx),
        }
    }
}
