//! Thin `FixedFlavor` (`Fixed`-point) aliases over `crate::generic`'s
//! engine — an opt-in, bit-reproducible simulation path for lockstep
//! networking/replay (see `meridian_numeric_core::Fixed`'s doc comment
//! for what that buys and why plain `f32` can't). See `crate::generic`'s
//! module doc for why this is just aliases, not a second copy of the
//! engine: since the SAT-based cuboid narrow phase is exactly as generic
//! as the sphere path, it comes along for free — no "sphere only" scope
//! limit needed here, unlike the old hand-duplicated
//! `physics-core::deterministic` module this replaces.
//!
//! Nothing here changes how [`crate::float::RigidBody`]/
//! [`crate::float::Integrator`]/etc. behave; it's a parallel API, not a
//! mode switch on those types. Use
//! [`meridian_gac_core::fixed_ga::FixedMotor3::to_float_lossy`] (and the
//! matching methods on `FixedVec3`/`FixedBivector3`) to hand a
//! deterministically-computed pose off to rendering/ECS/audio, which
//! stay entirely in `f32` regardless of which physics path produced it.

pub use meridian_gac_core::fixed_ga::FixedFlavor;

pub type ColliderShape = crate::generic::ColliderShape<FixedFlavor>;
pub type RigidBody = crate::generic::RigidBody<FixedFlavor>;
pub type BroadPhase = crate::generic::BroadPhase<FixedFlavor>;
pub type Contact = crate::generic::Contact<FixedFlavor>;
pub type NarrowPhase = crate::generic::NarrowPhase<FixedFlavor>;
pub type ConstraintSolver = crate::generic::ConstraintSolver<FixedFlavor>;
pub type Integrator = crate::generic::Integrator<FixedFlavor>;

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::fixed_ga::FixedMotor3;
    use meridian_gac_core::fixed_ga::FixedVec3;
    use meridian_numeric_core::Fixed;

    fn sphere(position: FixedVec3, velocity: FixedVec3, mass: f64, radius: f64) -> RigidBody {
        RigidBody {
            frame: FixedMotor3::translation(position),
            velocity,
            mass: Fixed::from_num(mass),
            shape: ColliderShape::Sphere {
                radius: Fixed::from_num(radius),
            },
            ..Default::default()
        }
    }

    fn fv3(x: f64, y: f64, z: f64) -> FixedVec3 {
        FixedVec3::new(Fixed::from_num(x), Fixed::from_num(y), Fixed::from_num(z))
    }

    #[test]
    fn broad_phase_finds_only_overlapping_pairs() {
        let bodies = vec![
            sphere(FixedVec3::ZERO, FixedVec3::ZERO, 1.0, 1.0),
            sphere(fv3(1.5, 0.0, 0.0), FixedVec3::ZERO, 1.0, 1.0),
            sphere(fv3(100.0, 0.0, 0.0), FixedVec3::ZERO, 1.0, 1.0),
        ];
        let mut broad = BroadPhase::new();
        let pairs = broad.find_candidate_pairs(&bodies);
        assert_eq!(pairs, &[(0, 1)]);
    }

    #[test]
    fn narrow_phase_reports_normal_pointing_from_a_to_b_and_correct_penetration() {
        let bodies = vec![
            sphere(FixedVec3::ZERO, FixedVec3::ZERO, 1.0, 1.0),
            sphere(fv3(1.5, 0.0, 0.0), FixedVec3::ZERO, 1.0, 1.0),
        ];
        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        // Fixed division rounds, so this is approximate, not exact equality.
        assert!((contact.normal.x.to_num() - 1.0).abs() < 1e-3);
        assert!(contact.normal.y.to_num().abs() < 1e-3);
        assert!(contact.normal.z.to_num().abs() < 1e-3);
        let penetration = contact.penetration.to_num();
        assert!((penetration - 0.5).abs() < 1e-3);
    }

    #[test]
    fn integrator_applies_gravity_and_moves_dynamic_bodies() {
        let mut bodies = vec![sphere(FixedVec3::ZERO, FixedVec3::ZERO, 1.0, 0.5)];
        Integrator::default().step(&mut bodies, Fixed::from_num(1.0));
        assert!((bodies[0].velocity.y.to_num() - -9.81).abs() < 1e-2);
        assert!(bodies[0].position().y.to_num() < 0.0);
    }

    #[test]
    fn integrator_never_moves_static_bodies() {
        let mut bodies = vec![sphere(FixedVec3::ZERO, FixedVec3::ZERO, 0.0, 1.0)];
        Integrator::default().step(&mut bodies, Fixed::from_num(1.0));
        assert_eq!(bodies[0].velocity, FixedVec3::ZERO);
        assert_eq!(bodies[0].position(), FixedVec3::ZERO);
    }

    #[test]
    fn full_step_ball_settles_on_static_floor_without_sinking_through() {
        let floor_radius = 50.0;
        let ball_radius = 0.5;
        let mut bodies = vec![
            sphere(
                fv3(0.0, -floor_radius, 0.0),
                FixedVec3::ZERO,
                0.0,
                floor_radius,
            ),
            sphere(fv3(0.0, 3.0, 0.0), FixedVec3::ZERO, 1.0, ball_radius),
        ];

        let integrator = Integrator::default();
        let solver = ConstraintSolver::new(Fixed::from_num(0.1));
        let mut broad = BroadPhase::new();
        let narrow = NarrowPhase::new();
        let dt = Fixed::from_num(1.0 / 60.0);

        for _ in 0..600 {
            integrator.step(&mut bodies, dt);
            let pairs = broad.find_candidate_pairs(&bodies).to_vec();
            for contact in narrow.generate_contacts(&bodies, &pairs) {
                solver.resolve(&mut bodies, &contact);
            }
        }

        let resting_height = bodies[1].position().y.to_num();
        assert!(
            (resting_height - ball_radius).abs() < 0.5,
            "ball should settle near the floor surface, got y={resting_height}"
        );
    }

    #[test]
    fn frame_f32_conversion_round_trips_position() {
        let body = sphere(fv3(1.5, -2.5, 3.5), FixedVec3::ZERO, 1.0, 0.5);
        let f32_frame = body.frame.to_float_lossy();
        let position = f32_frame.transform_point(meridian_gac_core::Vec3::ZERO);
        assert!((position.x - 1.5).abs() < 1e-3);
        assert!((position.y - -2.5).abs() < 1e-3);
        assert!((position.z - 3.5).abs() < 1e-3);
    }

    /// The actual point of this whole module: run the identical scenario
    /// (gravity, a collision with restitution, positional correction —
    /// every piece of the pipeline that matters) through two completely
    /// independent simulations and require the result to be bit-for-bit
    /// identical, not merely "close". `Fixed`'s `PartialEq` compares raw
    /// `i32` bits, so this is an exact equality check, unlike every other
    /// test in this file that (correctly) uses a tolerance.
    ///
    /// This proves reproducibility *within one process*, which is a
    /// necessary but not sufficient condition for the actual goal
    /// (cross-platform/cross-build reproducibility for lockstep
    /// networking) — a single test process can't independently verify
    /// behavior on different hardware. What it does verify: nothing in
    /// this pipeline introduces incidental nondeterminism of its own
    /// (uninitialized memory, hash-map iteration order, thread
    /// scheduling, ...) on top of `Fixed`'s own by-construction
    /// cross-platform guarantee.
    #[test]
    fn identical_inputs_produce_bit_identical_output_after_many_steps() {
        fn run() -> Vec<RigidBody> {
            let mut bodies = vec![
                sphere(fv3(0.0, -50.0, 0.0), FixedVec3::ZERO, 0.0, 50.0),
                sphere(fv3(0.3, 5.0, -0.2), fv3(0.1, 0.0, 0.05), 1.0, 0.5),
                sphere(fv3(-0.4, 8.0, 0.1), FixedVec3::ZERO, 1.0, 0.5),
            ];
            let integrator = Integrator::default();
            let solver = ConstraintSolver::new(Fixed::from_num(0.3));
            let mut broad = BroadPhase::new();
            let narrow = NarrowPhase::new();
            let dt = Fixed::from_num(1.0 / 60.0);

            for _ in 0..300 {
                integrator.step(&mut bodies, dt);
                let pairs = broad.find_candidate_pairs(&bodies).to_vec();
                for contact in narrow.generate_contacts(&bodies, &pairs) {
                    solver.resolve(&mut bodies, &contact);
                }
            }
            bodies
        }

        let run_a = run();
        let run_b = run();
        assert_eq!(
            run_a, run_b,
            "identical deterministic simulations must produce bit-identical state"
        );
    }
}
