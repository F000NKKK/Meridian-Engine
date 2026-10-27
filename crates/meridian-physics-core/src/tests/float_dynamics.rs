//! Float-flavor tests of the dynamics engine: solver, integrator, rigid-body inertia.

use meridian_gac_core::{Bivector3, Motor3, Vec3};

use super::float_fixtures::{cuboid, sphere};
use crate::float::*;
use core::f32::consts::PI;

#[test]
fn solver_separates_bodies_moving_toward_each_other() {
    let mut bodies = vec![
        sphere(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 1.0, 1.0),
        sphere(
            Vec3::new(1.5, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            1.0,
            1.0,
        ),
    ];
    let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
    ConstraintSolver::new(1.0).resolve(&mut bodies, &contact);

    let relative_velocity_after = bodies[1].velocity - bodies[0].velocity;
    assert!(
        relative_velocity_after.dot(contact.normal) >= 0.0,
        "bodies must no longer be closing after resolution"
    );
}

#[test]
fn solver_conserves_momentum_in_elastic_collision() {
    let mut bodies = vec![
        sphere(Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), 2.0, 1.0),
        sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0),
    ];
    let momentum_before = bodies[0].velocity * bodies[0].mass + bodies[1].velocity * bodies[1].mass;

    let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
    ConstraintSolver::new(1.0).resolve(&mut bodies, &contact);

    let momentum_after = bodies[0].velocity * bodies[0].mass + bodies[1].velocity * bodies[1].mass;
    assert!(
        (momentum_before - momentum_after).length() < 1e-4,
        "impulse resolution must conserve momentum: before {momentum_before:?}, after {momentum_after:?}"
    );
}

#[test]
fn solver_does_not_move_a_static_body() {
    let mut bodies = vec![
        sphere(Vec3::ZERO, Vec3::ZERO, 0.0, 1.0), // static: mass = 0
        sphere(
            Vec3::new(1.5, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            1.0,
            1.0,
        ),
    ];
    let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
    ConstraintSolver::new(0.0).resolve(&mut bodies, &contact);
    assert_eq!(
        bodies[0].velocity,
        Vec3::ZERO,
        "static body's velocity must never change"
    );
    assert_eq!(
        bodies[0].position(),
        Vec3::ZERO,
        "static body's position must never change"
    );
}

#[test]
fn sphere_sphere_contact_produces_no_spurious_spin() {
    let mut bodies = vec![
        sphere(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 1.0, 1.0),
        sphere(
            Vec3::new(1.5, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            1.0,
            1.0,
        ),
    ];
    let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
    ConstraintSolver::new(1.0).resolve(&mut bodies, &contact);

    assert_eq!(bodies[0].angular_velocity, Bivector3::ZERO);
    assert_eq!(bodies[1].angular_velocity, Bivector3::ZERO);
}

#[test]
fn integrator_applies_gravity_and_moves_dynamic_bodies() {
    let mut bodies = vec![sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 0.5)];
    Integrator::default().step(&mut bodies, 1.0);
    assert!((bodies[0].velocity.y - (-9.81)).abs() < 1e-4);
    assert!(bodies[0].position().y < 0.0, "body must have fallen");
}

#[test]
fn integrator_never_moves_static_bodies() {
    let mut bodies = vec![sphere(Vec3::ZERO, Vec3::ZERO, 0.0, 1.0)];
    Integrator::default().step(&mut bodies, 1.0);
    assert_eq!(bodies[0].velocity, Vec3::ZERO);
    assert_eq!(bodies[0].position(), Vec3::ZERO);
}

#[test]
fn integrator_rotates_a_spinning_body_via_bivector_exponential() {
    let mut bodies = vec![sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0)];
    bodies[0].angular_velocity = Bivector3::new(0.0, 0.0, PI); // spinning about Z at pi rad/s

    let dt = 1.0 / 120.0;
    for _ in 0..120 {
        Integrator::default().step(&mut bodies, dt);
    }
    // After 1 second at pi rad/s about Z, the body's local +X axis
    // should point along world -X (half turn).
    let local_x_direction = bodies[0].frame.transform_point(Vec3::X) - bodies[0].position();
    assert!(
        (local_x_direction - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-3,
        "expected a half-turn about Z, got local X direction {local_x_direction:?}"
    );
}

#[test]
fn cuboid_moment_of_inertia_matches_average_of_principal_moments() {
    let half_extents = Vec3::new(1.0, 2.0, 3.0);
    let body = cuboid(Motor3::identity(), Vec3::ZERO, 6.0, half_extents);
    let expected = (2.0 / 9.0)
        * 6.0
        * (half_extents.x * half_extents.x
            + half_extents.y * half_extents.y
            + half_extents.z * half_extents.z);
    assert!((body.moment_of_inertia() - expected).abs() < 1e-4);
}
