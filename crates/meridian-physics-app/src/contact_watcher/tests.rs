//! Tests of the contact watcher.

use meridian_gac::{Motor3, Vec3};
use meridian_physics_domain::{ColliderShape, RigidBody};

use super::*;
use crate::{PhysicsStepper, SequentialStepper};

fn sphere(at: Vec3, velocity: Vec3, mass: f32) -> RigidBody {
    RigidBody {
        frame: Motor3::translation(at),
        velocity,
        mass,
        shape: ColliderShape::Sphere { radius: 0.5 },
        ..Default::default()
    }
}

/// Steps `ticks` times and returns every event the watcher reported.
fn run(
    physics: &mut PhysicsSubsystem,
    watcher: &mut ContactWatcher,
    ticks: usize,
) -> Vec<ContactEvent> {
    let mut stepper = SequentialStepper;
    let mut events = Vec::new();
    for _ in 0..ticks {
        watcher.before_step(physics);
        stepper.step(physics, 1.0 / 60.0);
        events.extend(watcher.after_step(physics));
    }
    events
}

fn head_on_spheres() -> PhysicsSubsystem {
    let mut physics = PhysicsSubsystem::default();
    physics.bodies.push(sphere(
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
        1.0,
    ));
    physics.bodies.push(sphere(
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(-5.0, 0.0, 0.0),
        1.0,
    ));
    physics
}

#[test]
fn bodies_far_apart_report_nothing() {
    let mut physics = PhysicsSubsystem::default();
    physics
        .bodies
        .push(sphere(Vec3::new(-50.0, 0.0, 0.0), Vec3::ZERO, 1.0));
    physics
        .bodies
        .push(sphere(Vec3::new(50.0, 0.0, 0.0), Vec3::ZERO, 1.0));

    assert!(run(&mut physics, &mut ContactWatcher::new(0.0), 5).is_empty());
}

#[test]
fn a_head_on_collision_is_reported_once_with_its_closing_speed() {
    let mut physics = head_on_spheres();

    let events = run(&mut physics, &mut ContactWatcher::new(0.0), 120);

    assert_eq!(events.len(), 1, "{events:?}");
    let event = events[0];
    assert_eq!((event.a, event.b), (0, 1));
    assert!(
        event.impact_speed > 5.0,
        "closing at 10 u/s, got {}",
        event.impact_speed
    );
    assert!(
        event.normal.x > 0.9,
        "points from body 0 toward body 1: {:?}",
        event.normal
    );
}

#[test]
fn a_slow_touch_below_the_gate_is_silent_and_does_not_fire_later() {
    let mut physics = head_on_spheres();
    let mut watcher = ContactWatcher::new(100.0);

    assert!(run(&mut physics, &mut watcher, 120).is_empty());
}

#[test]
fn a_body_resting_on_another_is_not_reported_again() {
    let mut physics = PhysicsSubsystem::default();
    physics
        .bodies
        .push(sphere(Vec3::new(0.0, 0.0, 0.0), Vec3::ZERO, 0.0));
    physics.bodies.push(sphere(
        Vec3::new(0.0, 1.2, 0.0),
        Vec3::new(0.0, -3.0, 0.0),
        1.0,
    ));

    let events = run(&mut physics, &mut ContactWatcher::new(0.0), 240);

    assert!(events.len() <= 1, "landed once, then rested: {events:?}");
}

#[test]
fn separating_and_touching_again_is_a_new_event() {
    let mut physics = head_on_spheres();
    let mut watcher = ContactWatcher::new(0.0);
    let first = run(&mut physics, &mut watcher, 60);
    // Pull the bodies apart, let the watcher see the separation, then fire them together again.
    physics.bodies[0].frame = Motor3::translation(Vec3::new(-2.0, 0.0, 0.0));
    physics.bodies[1].frame = Motor3::translation(Vec3::new(2.0, 0.0, 0.0));
    physics.bodies[0].velocity = Vec3::ZERO;
    physics.bodies[1].velocity = Vec3::ZERO;
    run(&mut physics, &mut watcher, 4);
    physics.bodies[0].velocity = Vec3::new(5.0, 0.0, 0.0);
    physics.bodies[1].velocity = Vec3::new(-5.0, 0.0, 0.0);
    let second = run(&mut physics, &mut watcher, 60);

    assert_eq!((first.len(), second.len()), (1, 1));
}
