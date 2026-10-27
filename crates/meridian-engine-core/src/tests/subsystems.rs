//! Tests of the physics and audio subsystems.

use meridian_audio_core::{AttenuationModel, Channel, Emitter, Listener, Mixer, SpeakerLayout};
use meridian_gac_core::{Motor3, Vec3};
use meridian_physics_core::{ColliderShape, RigidBody};

use super::{falling_body, subsystem_pair};
use crate::*;

#[test]
fn audio_subsystem_mixes_audio_from_current_emitter_positions() {
    let mut audio = AudioSubsystem::new(
        Mixer::new(SpeakerLayout::stereo_headphones()).with_attenuation(AttenuationModel {
            reference_distance: 1000.0,
            rolloff: 1.0,
            max_distance: 1000.0,
        }),
    );
    audio.listener = Listener {
        frame: Motor3::identity(),
    };
    // Local +Z is "right" per audio-core's listener convention.
    audio.emitters.push((
        Emitter {
            frame: Motor3::translation(Vec3::new(0.0, 0.0, 5.0)),
        },
        1.0,
    ));

    let gains = audio.mix();
    let gain_of = |channel: Channel| {
        gains
            .iter()
            .find(|(c, _)| *c == channel)
            .map(|(_, g)| *g)
            .unwrap_or(0.0)
    };
    assert!(gain_of(Channel::Right) > 0.99);
    assert!(gain_of(Channel::Left) < 1e-3);
}

#[test]
fn physics_subsystem_step_resolves_a_resting_contact() {
    let mut physics = PhysicsSubsystem::default();
    physics.bodies.push(RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, -50.0, 0.0)),
        mass: 0.0, // static floor
        shape: ColliderShape::Sphere { radius: 50.0 },
        ..Default::default()
    });
    physics.bodies.push(falling_body());

    for _ in 0..600 {
        physics.step(1.0 / 60.0);
    }

    let resting_height = physics.bodies[1].position().y;
    assert!(
        (resting_height - 0.5).abs() < 0.5,
        "ball should settle near the floor surface, got y={resting_height}"
    );
}

/// A box-on-box manifold (up to 4 contact points, unlike the
/// single-point sphere case above) — regression coverage for the bug
/// `step`'s doc comment describes: the original one-`resolve()`-
/// call-per-contact shape over-applied positional correction on
/// every relaxation-worthy contact set, bouncing a settled box
/// up/down and eventually clipping it through the floor. A sphere
/// never exercised this (always exactly one contact point), which is
/// exactly why the bug went unnoticed in this method even after
/// `examples/physic_figures` independently found and fixed it in its
/// own hand-rolled stepping — see `meridian-physics-core::float`'s
/// own `cuboid_settles_without_runaway_spin` test for the
/// equivalent, non-centralized version of this same assertion.
#[test]
fn physics_subsystem_step_settles_a_box_without_bouncing_or_sinking() {
    use meridian_physics_core::ConstraintSolver;

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

    let mut min_height_after_landing = f32::MAX;
    let mut max_height_after_landing = f32::MIN;
    for step in 0..600 {
        physics.step(1.0 / 60.0);
        if step > 200 {
            let height = physics.bodies[1].position().y;
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

/// A single [`PhysicsStepStage`] run through [`Runtime::tick`] must
/// match calling `PhysicsSubsystem::step` directly — the runtime
/// mechanism must not change what a stage does, only when it runs.
#[test]
fn physics_step_stage_matches_direct_step_call() {
    let (mut physics, audio) = subsystem_pair();
    physics.bodies.push(falling_body());
    let mut runtime = Runtime::with_worker_count(physics, audio, 2);
    runtime.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, PhysicsDispatchMode::Sequential),
    );
    let state = runtime.state().clone();

    runtime.tick();

    let velocity_after_tick = state.physics().bodies[0].velocity;

    let mut expected = PhysicsSubsystem::default();
    expected.bodies.push(falling_body());
    expected.step(1.0 / 60.0);

    assert_eq!(velocity_after_tick, expected.bodies[0].velocity);
}
