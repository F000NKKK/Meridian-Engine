//! Thin `FloatFlavor` (`f32`) aliases over `crate::generic`'s engine —
//! the default, everyday physics path. See `crate::generic`'s module doc
//! for why this is just aliases, not a second copy of the engine.

pub use meridian_gac_core::float_ga::FloatFlavor;

pub type ColliderShape = crate::generic::ColliderShape<FloatFlavor>;
pub type RigidBody = crate::generic::RigidBody<FloatFlavor>;
pub type BroadPhase = crate::generic::BroadPhase<FloatFlavor>;
pub type Contact = crate::generic::Contact<FloatFlavor>;
pub type NarrowPhase = crate::generic::NarrowPhase<FloatFlavor>;
pub type ConstraintSolver = crate::generic::ConstraintSolver<FloatFlavor>;
pub type Integrator = crate::generic::Integrator<FloatFlavor>;

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;
    use meridian_gac_core::{Bivector3, Motor3, Vec3};

    fn sphere(position: Vec3, velocity: Vec3, mass: f32, radius: f32) -> RigidBody {
        RigidBody {
            frame: Motor3::translation(position),
            velocity,
            mass,
            shape: ColliderShape::Sphere { radius },
            ..Default::default()
        }
    }

    fn cuboid(frame: Motor3, velocity: Vec3, mass: f32, half_extents: Vec3) -> RigidBody {
        RigidBody {
            frame,
            velocity,
            mass,
            shape: ColliderShape::Cuboid { half_extents },
            ..Default::default()
        }
    }

    fn assert_vec3_approx(a: Vec3, b: Vec3) {
        assert!((a - b).length() < 1e-4, "expected {b:?}, got {a:?}");
    }

    #[test]
    fn aabb_overlap_basic_cases() {
        use meridian_gac_core::Aabb;
        let a = Aabb::from_sphere(Vec3::ZERO, 1.0);
        let touching = Aabb::from_sphere(Vec3::new(2.0, 0.0, 0.0), 1.0);
        let separate = Aabb::from_sphere(Vec3::new(10.0, 0.0, 0.0), 1.0);
        assert!(
            a.overlaps(&touching),
            "boxes sharing a boundary count as overlapping"
        );
        assert!(!a.overlaps(&separate));
    }

    #[test]
    fn broad_phase_finds_only_overlapping_pairs() {
        let bodies = vec![
            sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0),
            sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0), // overlaps body 0
            sphere(Vec3::new(100.0, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0), // far away
        ];
        let mut broad = BroadPhase::new();
        let pairs = broad.find_candidate_pairs(&bodies);
        assert_eq!(pairs, &[(0, 1)]);
    }

    #[test]
    fn narrow_phase_reports_normal_pointing_from_a_to_b_and_correct_penetration() {
        let bodies = vec![
            sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0),
            sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0),
        ];
        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        assert_eq!(contact.normal, Vec3::new(1.0, 0.0, 0.0));
        assert!(
            (contact.penetration - 0.5).abs() < 1e-5,
            "spheres of radius 1 each, centers 1.5 apart -> 0.5 overlap"
        );
    }

    #[test]
    fn narrow_phase_returns_none_when_not_touching() {
        let bodies = vec![
            sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0),
            sphere(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0),
        ];
        assert!(NarrowPhase::new().test_pair(&bodies, 0, 1).is_none());
    }

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
        let momentum_before =
            bodies[0].velocity * bodies[0].mass + bodies[1].velocity * bodies[1].mass;

        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        ConstraintSolver::new(1.0).resolve(&mut bodies, &contact);

        let momentum_after =
            bodies[0].velocity * bodies[0].mass + bodies[1].velocity * bodies[1].mass;
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

    #[test]
    fn narrow_phase_sphere_vs_cuboid_reports_correct_penetration_and_normal() {
        let floor = cuboid(
            Motor3::translation(Vec3::new(0.0, -5.0, 0.0)),
            Vec3::ZERO,
            0.0,
            Vec3::new(10.0, 5.0, 10.0),
        );
        let ball = sphere(Vec3::new(0.0, 0.3, 0.0), Vec3::ZERO, 1.0, 0.5);
        let bodies = vec![floor, ball];

        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        assert_vec3_approx(contact.normal, Vec3::new(0.0, 1.0, 0.0));
        assert!((contact.penetration - 0.2).abs() < 1e-4);
        assert_vec3_approx(contact.point, Vec3::ZERO);
    }

    #[test]
    fn narrow_phase_sphere_vs_cuboid_returns_none_when_separated() {
        let floor = cuboid(
            Motor3::identity(),
            Vec3::ZERO,
            0.0,
            Vec3::new(1.0, 1.0, 1.0),
        );
        let ball = sphere(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO, 1.0, 0.5);
        let bodies = vec![floor, ball];
        assert!(NarrowPhase::new().test_pair(&bodies, 0, 1).is_none());
    }

    #[test]
    fn narrow_phase_sphere_vs_cuboid_handles_embedded_sphere_without_panicking() {
        let box_body = cuboid(
            Motor3::identity(),
            Vec3::ZERO,
            0.0,
            Vec3::new(1.0, 1.0, 1.0),
        );
        let ball = sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 0.3);
        let bodies = vec![box_body, ball];

        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        assert!(contact.penetration > 0.0 && contact.penetration.is_finite());
        assert!((contact.normal.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn narrow_phase_cuboid_vs_cuboid_reports_least_penetration_axis() {
        let a = cuboid(
            Motor3::identity(),
            Vec3::ZERO,
            1.0,
            Vec3::new(1.0, 1.0, 1.0),
        );
        let b = cuboid(
            Motor3::translation(Vec3::new(1.5, 0.0, 0.0)),
            Vec3::ZERO,
            1.0,
            Vec3::new(1.0, 1.0, 1.0),
        );
        let bodies = vec![a, b];

        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        assert_vec3_approx(contact.normal, Vec3::new(1.0, 0.0, 0.0));
        assert!((contact.penetration - 0.5).abs() < 1e-4);
    }

    #[test]
    fn narrow_phase_cuboid_vs_cuboid_returns_none_when_separated() {
        let a = cuboid(
            Motor3::identity(),
            Vec3::ZERO,
            1.0,
            Vec3::new(1.0, 1.0, 1.0),
        );
        let b = cuboid(
            Motor3::translation(Vec3::new(5.0, 0.0, 0.0)),
            Vec3::ZERO,
            1.0,
            Vec3::new(1.0, 1.0, 1.0),
        );
        let bodies = vec![a, b];
        assert!(NarrowPhase::new().test_pair(&bodies, 0, 1).is_none());
    }

    #[test]
    fn narrow_phase_cuboid_vs_cuboid_handles_rotated_boxes() {
        let a = cuboid(
            Motor3::identity(),
            Vec3::ZERO,
            1.0,
            Vec3::new(1.0, 1.0, 1.0),
        );
        let overlapping_b = cuboid(
            Motor3::rotation(Vec3::Y, PI / 4.0)
                .compose(Motor3::translation(Vec3::new(1.5, 0.0, 0.0))),
            Vec3::ZERO,
            1.0,
            Vec3::new(1.0, 1.0, 1.0),
        );
        let bodies = vec![a, overlapping_b];
        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        assert!(contact.penetration > 0.0 && contact.penetration.is_finite());
        assert!((contact.normal.length() - 1.0).abs() < 1e-4);

        let separated_b = cuboid(
            Motor3::rotation(Vec3::Y, PI / 4.0)
                .compose(Motor3::translation(Vec3::new(10.0, 0.0, 0.0))),
            Vec3::ZERO,
            1.0,
            Vec3::new(1.0, 1.0, 1.0),
        );
        let bodies = vec![a, separated_b];
        assert!(NarrowPhase::new().test_pair(&bodies, 0, 1).is_none());
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
}
