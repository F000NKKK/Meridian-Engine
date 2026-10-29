//! The port that advances a physics world by one timestep.

use crate::PhysicsSubsystem;

/// Advances every body of a [`PhysicsSubsystem`] by `dt`: integrate, relax
/// contacts over [`RELAXATION_ITERATIONS`](crate::RELAXATION_ITERATIONS)
/// velocity-only passes, then one positional-correction pass.
///
/// The runtime owns the *when*; an implementation owns the *how*:
/// [`SequentialStepper`](crate::SequentialStepper) is a plain CPU loop,
/// and `meridian-physics-wgpu` implements the same contract with
/// batched compute kernels. Every implementation must pass
/// `testing::run_stepper_contract` (available with the `testing` feature).
pub trait PhysicsStepper: Send {
    /// Steps `physics` by `dt` seconds.
    fn step(&mut self, physics: &mut PhysicsSubsystem, dt: f32);
}
