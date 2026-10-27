//! The physics world state the runtime steps each frame.

use meridian_physics_domain::{BroadPhase, ConstraintSolver, Integrator, NarrowPhase, RigidBody};

/// Velocity-relaxation pass count for [`SequentialStepper`](crate::SequentialStepper) — chosen
/// because a box/pyramid face-face manifold (up to 4 points) needs several
/// passes before each point's impulse is computed against the others'
/// relaxed (not stale pre-solve) velocity. Every [`PhysicsStepper`](crate::PhysicsStepper)
/// uses the same count, so they stay comparable.
pub const RELAXATION_ITERATIONS: u32 = 4;

/// `physics-core`'s body list plus its broad/narrow-phase and
/// solver/integrator: everything a [`PhysicsStepper`](crate::PhysicsStepper) advances by one
/// timestep. The state is plain data owned by whoever composes the
/// application; no lock lives here.
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
