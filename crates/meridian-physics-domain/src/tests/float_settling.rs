//! Float-flavor full-step tests: bodies settling on static floors.

use meridian_gac::{Motor3, Vec3};

use super::float_fixtures::{cuboid, sphere};
use crate::float::*;

#[test]
fn full_step_ball_settles_on_static_floor_without_sinking_through() {
    let floor_radius = 50.0;
    let ball_radius = 0.5;
    let mut bodies = vec![
        sphere(
            Vec3::new(0.0, -floor_radius, 0.0),
            Vec3::ZERO,
            0.0,
            floor_radius,
        ),
        sphere(Vec3::new(0.0, 3.0, 0.0), Vec3::ZERO, 1.0, ball_radius),
    ];

    let integrator = Integrator::default();
    let solver = ConstraintSolver::new(0.1);
    let mut broad = BroadPhase::new();
    let narrow = NarrowPhase::new();
    let dt = 1.0 / 60.0;

    for _ in 0..600 {
        integrator.step(&mut bodies, dt);
        let pairs = broad.find_candidate_pairs(&bodies).to_vec();
        for contact in narrow.generate_contacts(&bodies, &pairs) {
            solver.resolve(&mut bodies, &contact);
        }
    }

    let resting_height = bodies[1].position().y;
    let expected = ball_radius;
    assert!(
        (resting_height - expected).abs() < 0.5,
        "ball should settle near the floor surface, got y={resting_height}, expected near {expected}"
    );
}

#[test]
fn full_step_cuboid_settles_on_static_cuboid_floor_without_sinking_through() {
    let floor = cuboid(
        Motor3::translation(Vec3::new(0.0, -5.0, 0.0)),
        Vec3::ZERO,
        0.0,
        Vec3::new(10.0, 5.0, 10.0),
    );
    let falling = cuboid(
        Motor3::translation(Vec3::new(0.0, 3.0, 0.0)),
        Vec3::ZERO,
        1.0,
        Vec3::new(0.5, 0.5, 0.5),
    );
    let mut bodies = vec![floor, falling];

    let integrator = Integrator::default();
    let solver = ConstraintSolver::new(0.1);
    let mut broad = BroadPhase::new();
    let narrow = NarrowPhase::new();
    let dt = 1.0 / 60.0;

    for _ in 0..600 {
        integrator.step(&mut bodies, dt);
        let pairs = broad.find_candidate_pairs(&bodies).to_vec();
        for contact in narrow.generate_contacts(&bodies, &pairs) {
            solver.resolve(&mut bodies, &contact);
        }
    }

    let resting_height = bodies[1].position().y;
    assert!(
        (resting_height - 0.5).abs() < 0.5,
        "box should settle near the floor surface, got y={resting_height}"
    );
}

/// Regression test for the "cube/pyramid spin hysterically on
/// touchdown" bug: a box-box manifold's per-corner torque isn't a
/// coupled solve (see `Contact::suppress_angular_response`'s doc
/// comment), so before that flag existed this box's
/// `angular_velocity` diverged instead of settling. A flat box
/// dropped with zero initial spin onto a flat static floor has no
/// physical reason to ever spin fast — this runs the *exact* same
/// `examples/examples/physic_figures.rs` loop shape (integrate,
/// broad, narrow, resolve, with friction) headlessly in a fraction
/// of a second, so this bug (and any regression of it) is caught by
/// `cargo test -p meridian-physics-domain` without launching the
/// windowed example at all.
#[test]
fn cuboid_settles_without_runaway_spin() {
    let floor = cuboid(
        Motor3::translation(Vec3::new(0.0, -0.5, 0.0)),
        Vec3::ZERO,
        0.0,
        Vec3::new(14.0, 0.5, 14.0),
    );
    let falling = cuboid(
        Motor3::translation(Vec3::new(0.0, 3.0, 0.0)),
        Vec3::ZERO,
        1.0,
        Vec3::new(0.6, 0.6, 0.6),
    );
    let mut bodies = vec![floor, falling];

    let integrator = Integrator::default();
    // Matches `examples/examples/physic_figures.rs`: restitution 0
    // (a settled body must not bounce at all — the
    // `restitution_velocity_threshold` default alone already
    // suppresses bounce off residual gravity velocity, but `0` here
    // means there's no ambiguity about intent for this test).
    let solver = ConstraintSolver::new(0.0).with_friction(0.6);
    let mut broad = BroadPhase::new();
    let narrow = NarrowPhase::new();
    let mut max_angular_speed_after_landing: f32 = 0.0;
    let mut min_height_after_landing = f32::MAX;
    let mut max_height_after_landing = f32::MIN;
    for step in 0..600 {
        simulate_step(&integrator, &solver, &mut broad, &narrow, &mut bodies);
        // Give the box a couple hundred steps to make first contact
        // and let any initial-impact wobble die down before tracking
        // the "should now be at rest" window.
        if step > 200 {
            max_angular_speed_after_landing =
                max_angular_speed_after_landing.max(bodies[1].angular_velocity.length());
            let height = bodies[1].position().y;
            min_height_after_landing = min_height_after_landing.min(height);
            max_height_after_landing = max_height_after_landing.max(height);
        }
        assert!(
            bodies[1].position().y.is_finite() && bodies[1].angular_velocity.length().is_finite(),
            "body must never diverge to NaN/infinity (step {step})"
        );
    }

    assert!(
        max_angular_speed_after_landing < 1.0,
        "a flat box dropped with no initial spin should settle, not keep spinning \
         (max angular speed after landing: {max_angular_speed_after_landing} rad/s)"
    );
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

/// One tick as `examples/examples/physic_figures.rs` runs it.
fn simulate_step(
    integrator: &Integrator,
    solver: &ConstraintSolver,
    broad: &mut BroadPhase,
    narrow: &NarrowPhase,
    bodies: &mut [RigidBody],
) {
    let dt = 1.0 / 60.0;
    integrator.step(bodies, dt);
    // Velocity-only relaxation passes, then positional
    // correction exactly once — matching
    // `examples/examples/physic_figures.rs`'s loop. Calling the
    // combined `resolve` (velocity + position) once per
    // relaxation pass instead applied the positional push
    // several times per tick, which is the "cube/pyramid bounce
    // up/down and clip through the floor" bug this test's
    // height-stability assertion below catches — see
    // `ConstraintSolver::resolve`'s doc comment.
    for _ in 0..4 {
        let pairs = broad.find_candidate_pairs(bodies).to_vec();
        for contact in narrow.generate_contacts(bodies, &pairs) {
            solver.resolve_velocity(bodies, &contact);
        }
    }
    let pairs = broad.find_candidate_pairs(bodies).to_vec();
    for contact in narrow.generate_contacts(bodies, &pairs) {
        solver.apply_positional_correction(bodies, &contact);
    }
}
