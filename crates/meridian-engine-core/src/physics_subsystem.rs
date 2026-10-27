//! The physics state the runtime steps each frame.

use meridian_physics_core::{BroadPhase, ConstraintSolver, Integrator, NarrowPhase, RigidBody};

/// Velocity-relaxation pass count for [`PhysicsSubsystem::step`] — matches
/// `examples/physic_figures`' own tuned value, chosen because a
/// box/pyramid face-face manifold (up to 4 points) needs several passes
/// before each point's impulse is computed against the others' relaxed
/// (not stale pre-solve) velocity.
pub(crate) const RELAXATION_ITERATIONS: u32 = 4;

/// `physics-core`'s body list plus its broad/narrow-phase and
/// solver/integrator — everything [`PhysicsSubsystem::step`] needs,
/// bundled as one independently-lockable unit inside [`RuntimeState`](crate::RuntimeState) so
/// a [`Stage`](crate::Stage) that only touches physics never blocks on
/// [`AudioSubsystem`](crate::AudioSubsystem)'s or `World`'s lock. `engine-core` still owns the
/// actual stepping *logic* here (rule 7: this is where cross-`*-core`
/// domain logic lives, not a downstream orchestration crate re-deriving
/// it) — only *when* it runs is `Stage`/`Runtime`'s concern.
#[derive(Debug)]
pub struct PhysicsSubsystem {
    pub bodies: Vec<RigidBody>,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub solver: ConstraintSolver,
    pub integrator: Integrator,
}

impl Default for PhysicsSubsystem {
    fn default() -> Self {
        Self {
            bodies: Vec::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            solver: ConstraintSolver::default(),
            integrator: Integrator::default(),
        }
    }
}

impl PhysicsSubsystem {
    /// Advances every body by `dt`: integrate, then relax contacts over
    /// `RELAXATION_ITERATIONS` velocity-only passes before a single
    /// final positional correction pass. **Not** one `resolve()` call per
    /// contact — that was this method's original shape (back when it
    /// lived directly on `SubsystemManager`), and it carries the exact
    /// "cube/pyramid bounces up/down and clips through the floor" bug
    /// `ConstraintSolver::resolve`'s own doc comment describes: a
    /// box/pyramid manifold is up to 4 contact points sharing one normal,
    /// and a single combined velocity+positional pass over all of them
    /// pushes the body's position by the same correction several times
    /// per tick. A single sphere-sphere contact (this method's own
    /// regression test) never exhibited it, so the bug went unnoticed
    /// here even after `examples/physic_figures` independently
    /// discovered and fixed it in its own hand-rolled physics stepping —
    /// see docs/roadmap.md's `Runtime`-adoption entry for that history.
    /// This method now mirrors `physic_figures::PhysicsRig::step`'s
    /// proven-correct shape exactly, centralized here so every caller
    /// gets multi-point-manifold stability for free instead of
    /// rediscovering it.
    pub fn step(&mut self, dt: f32) {
        self.integrator.step(&mut self.bodies, dt);

        for _ in 0..RELAXATION_ITERATIONS {
            let pairs = self.broad_phase.find_candidate_pairs(&self.bodies).to_vec();
            for contact in self.narrow_phase.generate_contacts(&self.bodies, &pairs) {
                self.solver.resolve_velocity(&mut self.bodies, &contact);
            }
        }
        // Positional correction exactly once per tick, against the final
        // (velocity-relaxed) contact set — see `ConstraintSolver::resolve`'s
        // doc comment for why calling it more than once per tick over-corrects.
        let pairs = self.broad_phase.find_candidate_pairs(&self.bodies).to_vec();
        for contact in self.narrow_phase.generate_contacts(&self.bodies, &pairs) {
            self.solver
                .apply_positional_correction(&mut self.bodies, &contact);
        }
    }
}
