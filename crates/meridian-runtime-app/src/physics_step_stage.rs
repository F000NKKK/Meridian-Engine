//! The stage that steps physics.

use meridian_physics_app::{PhysicsStepper, PhysicsSubsystem};

use crate::{Shared, Stage};

/// Built-in stage that advances a shared [`PhysicsSubsystem`] by a fixed
/// `dt` every run, through whichever [`PhysicsStepper`] the composition
/// root chose: the sequential CPU loop (`SequentialStepper`) or the batched
/// compute stepper of `meridian-physics-wgpu`. Same algorithm, a
/// different dispatch shape; every stepper passes one contract suite.
pub struct PhysicsStepStage {
    pub dt: f32,
    stepper: Box<dyn PhysicsStepper>,
    physics: Shared<PhysicsSubsystem>,
}

impl PhysicsStepStage {
    /// A stage stepping `physics` by `dt` with `stepper`.
    #[must_use]
    pub fn new(
        dt: f32,
        stepper: impl PhysicsStepper + 'static,
        physics: Shared<PhysicsSubsystem>,
    ) -> Self {
        Self {
            dt,
            stepper: Box::new(stepper),
            physics,
        }
    }
}

impl std::fmt::Debug for PhysicsStepStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhysicsStepStage")
            .field("dt", &self.dt)
            .finish_non_exhaustive()
    }
}

impl Stage for PhysicsStepStage {
    fn run(&mut self) {
        self.stepper.step(&mut self.physics.lock(), self.dt);
    }
}
