//! The physics runtime stepped every frame.

use meridian_sdk::{
    BatchedStepper, ConstraintSolver, Motor3, PhysicsStepStage, PhysicsSubsystem, RigidBody,
    Runtime, Shared, StageId, cpu_context, parallel_runtime,
};

use crate::SceneEntity;

const PHYSICS_DT: f32 = 1.0 / 60.0;

/// `0`: a settled body must not bounce at all. Combined with
/// `ConstraintSolver`'s `restitution_velocity_threshold` (which already
/// suppresses bounce below a small closing speed regardless of this
/// value), this makes landing fully inelastic — no perpetual "settles,
/// then bounces off residual gravity velocity" jitter, ever.
const SOLVER_RESTITUTION: f32 = 0.0;

/// Coulomb friction coefficient — without this, resting bodies had
/// nothing slowing lateral sliding, so any small rotational/positional
/// jitter (see `NarrowPhase`'s box-box manifold) could slide the box
/// across the floor indefinitely instead of settling.
const SOLVER_FRICTION: f32 = 0.6;

/// Fixed-timestep driver around a real `meridian_sdk::Runtime` running
/// one registered [`PhysicsStepStage`] driving a `BatchedStepper`
/// — the physics stepping
/// itself (integrate/relax-contacts/resolve, batched through
/// `physics-wgpu`'s kernels rather than a plain sequential loop) is
/// the stage's job, not this file's; see that stage's own doc comment for
/// the multi-point-manifold relaxation semantics it preserves exactly.
/// What's left at this layer is
/// deliberately application policy, not engine plumbing: the render
/// loop's `frame_dt` varies with frame rate, but the solver is only
/// validated at a constant [`PHYSICS_DT`], so this accumulates
/// wall-clock time and calls `Runtime::tick_only` (just the physics
/// stage — see that method's own doc for why not plain `tick`) once per
/// whole [`PHYSICS_DT`] increment, capped so a stall (e.g. window drag)
/// can't spiral into running hundreds of catch-up ticks at once. Also
/// owns the `Runtime` the render stage gets registered onto in
/// [`AppHandler::on_ready`] — physics and render share one `Runtime`,
/// per this module's own top-level doc.
pub(crate) struct PhysicsRig {
    pub(crate) runtime: Runtime,
    /// The physics world, shared with the render stage's scene closure.
    pub(crate) physics: Shared<PhysicsSubsystem>,
    pub(crate) physics_stage: StageId,
    accumulator: f32,
}

impl PhysicsRig {
    pub(crate) fn new(scene: &[SceneEntity]) -> Self {
        let bodies = scene
            .iter()
            .map(|entity| RigidBody {
                frame: Motor3::translation(entity.position),
                mass: entity.mass,
                shape: entity.collider,
                ..Default::default()
            })
            .collect();

        let physics = Shared::new(PhysicsSubsystem {
            bodies,
            solver: ConstraintSolver::new(SOLVER_RESTITUTION).with_friction(SOLVER_FRICTION),
            ..Default::default()
        });

        let mut runtime = parallel_runtime();
        // `BatchedStepper`, not `SequentialStepper` — this example is the
        // live proof that the batched-dispatch physics path
        // (docs/roadmap.md's "Scaling beyond a tech demo" item 4) is a
        // real drop-in swap, not just unit-tested in isolation.
        let physics_stage = runtime.add_stage(
            "physics",
            &[],
            PhysicsStepStage::new(
                PHYSICS_DT,
                BatchedStepper::new(cpu_context()),
                physics.clone(),
            ),
        );

        Self {
            runtime,
            physics,
            physics_stage,
            accumulator: 0.0,
        }
    }

    pub(crate) fn step(&mut self, frame_dt: f32) {
        self.accumulator += frame_dt;
        let mut steps = 0;
        while self.accumulator >= PHYSICS_DT && steps < 8 {
            self.runtime.tick_only(&[self.physics_stage]);
            self.accumulator -= PHYSICS_DT;
            steps += 1;
        }
    }
}
