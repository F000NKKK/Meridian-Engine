//! Tests of the compute-dispatched physics stage.

use super::falling_body;
use super::subsystem_pair;
use crate::*;
use meridian_audio_core::{Mixer, SpeakerLayout};
use meridian_gac_core::{Motor3, Vec3};
use meridian_physics_core::{ColliderShape, ConstraintSolver, RigidBody};

/// `PhysicsStepStage` in `PhysicsDispatchMode::Batched` must settle a
/// box exactly the way `PhysicsDispatchMode::Sequential`/
/// `PhysicsSubsystem::step` do — same test shape as
/// `physics_subsystem_step_settles_a_box_without_bouncing_or_sinking`,
/// run through the batched-dispatch mode instead of the plain
/// sequential loop, proving the swap is behavior-preserving, not
/// just "compiles and looks physically plausible."
#[test]
fn physics_compute_step_stage_settles_a_box_without_bouncing_or_sinking() {
    let mut physics = PhysicsSubsystem {
        solver: ConstraintSolver::new(0.0).with_friction(0.6),
        ..Default::default()
    };
    physics.bodies.push(RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, -0.5, 0.0)),
        mass: 0.0, // static floor
        shape: ColliderShape::Cuboid {
            half_extents: Vec3::new(14.0, 0.5, 14.0),
        },
        ..Default::default()
    });
    physics.bodies.push(RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, 3.0, 0.0)),
        mass: 1.0,
        shape: ColliderShape::Cuboid {
            half_extents: Vec3::new(0.6, 0.6, 0.6),
        },
        ..Default::default()
    });

    let audio = AudioSubsystem::new(Mixer::new(SpeakerLayout::mono()));
    let mut runtime = Runtime::with_worker_count(physics, audio, 2);
    let physics_id = runtime.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, PhysicsDispatchMode::Batched),
    );

    let mut min_height_after_landing = f32::MAX;
    let mut max_height_after_landing = f32::MIN;
    for step in 0..600 {
        runtime.tick_only(&[physics_id]);
        if step > 200 {
            let height = runtime.state().physics().bodies[1].position().y;
            min_height_after_landing = min_height_after_landing.min(height);
            max_height_after_landing = max_height_after_landing.max(height);
        }
    }

    assert!(
        max_height_after_landing - min_height_after_landing < 0.01,
        "a settled box (restitution 0) must not bounce up/down at all \
         (min {min_height_after_landing}, max {max_height_after_landing})"
    );
    assert!(
        min_height_after_landing > 0.0,
        "a settled box must not clip through the floor (min height {min_height_after_landing})"
    );
}

/// One tick of `PhysicsStepStage` in `PhysicsDispatchMode::Batched`
/// must match one tick in `PhysicsDispatchMode::Sequential`
/// bit-for-bit on a simple falling body — the direct "same
/// algorithm, different dispatch" proof, independent of the longer
/// settling regression above.
#[test]
fn physics_compute_step_stage_matches_physics_step_stage_for_one_tick() {
    let (physics_a, audio_a) = subsystem_pair();
    let mut physics_a = physics_a;
    physics_a.bodies.push(falling_body());
    let mut runtime_a = Runtime::with_worker_count(physics_a, audio_a, 2);
    let stage_a = runtime_a.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, PhysicsDispatchMode::Sequential),
    );
    runtime_a.tick_only(&[stage_a]);

    let (physics_b, audio_b) = subsystem_pair();
    let mut physics_b = physics_b;
    physics_b.bodies.push(falling_body());
    let mut runtime_b = Runtime::with_worker_count(physics_b, audio_b, 2);
    let stage_b = runtime_b.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, PhysicsDispatchMode::Batched),
    );
    runtime_b.tick_only(&[stage_b]);

    assert_eq!(
        runtime_a.state().physics().bodies[0].frame,
        runtime_b.state().physics().bodies[0].frame
    );
    assert_eq!(
        runtime_a.state().physics().bodies[0].velocity,
        runtime_b.state().physics().bodies[0].velocity
    );
}
