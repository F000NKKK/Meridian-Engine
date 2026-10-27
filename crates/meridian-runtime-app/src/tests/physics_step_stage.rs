//! Tests of the physics step stage.

use std::sync::Arc;

use meridian_gac::{Motor3, Vec3};
use meridian_physics_app::{PhysicsStepper, PhysicsSubsystem, SequentialStepper};
use meridian_physics_domain::{ColliderShape, RigidBody};

use crate::testing::InlineExecutor;
use crate::*;

fn falling_body() -> RigidBody {
    RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, 10.0, 0.0)),
        mass: 1.0,
        shape: ColliderShape::Sphere { radius: 0.5 },
        ..Default::default()
    }
}

/// A [`PhysicsStepStage`] run through `Runtime::tick` must match calling
/// the stepper directly — the runtime must not change what a stage does,
/// only when it runs.
#[test]
fn physics_step_stage_matches_a_direct_step_call() {
    let physics = Shared::new(PhysicsSubsystem::default());
    physics.lock().bodies.push(falling_body());
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    runtime.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, SequentialStepper, physics.clone()),
    );

    runtime.tick();

    let mut expected = PhysicsSubsystem::default();
    expected.bodies.push(falling_body());
    SequentialStepper.step(&mut expected, 1.0 / 60.0);

    assert_eq!(
        physics.lock().bodies[0].velocity,
        expected.bodies[0].velocity
    );
}
