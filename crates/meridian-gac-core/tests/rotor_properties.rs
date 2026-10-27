//! Property tests for rotors, motors and vectors: invariants that must hold
//! for every axis, angle and point, not just hand-picked ones.

use meridian_gac_core::{Motor3, Rotor, Vec3};
use proptest::prelude::*;

const TOLERANCE: f32 = 2e-3;

fn component() -> impl Strategy<Value = f32> {
    -10.0f32..10.0
}

fn vector() -> impl Strategy<Value = Vec3> {
    (component(), component(), component()).prop_map(|(x, y, z)| Vec3::new(x, y, z))
}

fn axis() -> impl Strategy<Value = Vec3> {
    vector().prop_filter("axis must be non-degenerate", |v| v.length() > 0.1)
}

fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).length() <= TOLERANCE * (1.0 + a.length().max(b.length()))
}

proptest! {
    #[test]
    fn rotation_preserves_length(axis in axis(), angle in -6.3f32..6.3, v in vector()) {
        let rotated = Rotor::from_axis_angle(axis, angle).transform_vector(v);
        prop_assert!((rotated.length() - v.length()).abs() <= TOLERANCE * (1.0 + v.length()));
    }

    #[test]
    fn reverse_rotor_undoes_rotation(axis in axis(), angle in -6.3f32..6.3, v in vector()) {
        let rotor = Rotor::from_axis_angle(axis, angle);
        prop_assert!(close(rotor.reverse().transform_vector(rotor.transform_vector(v)), v));
    }

    #[test]
    fn composition_applies_left_operand_first(
        a in axis(), b in axis(), angle_a in -3.1f32..3.1, angle_b in -3.1f32..3.1, v in vector(),
    ) {
        let first = Rotor::from_axis_angle(a, angle_a);
        let second = Rotor::from_axis_angle(b, angle_b);
        let stepwise = second.transform_vector(first.transform_vector(v));
        prop_assert!(close(first.compose(second).transform_vector(v), stepwise));
    }

    #[test]
    fn motor_inverse_round_trips_points(
        axis in axis(), angle in -3.1f32..3.1, t in vector(), p in vector(),
    ) {
        let motor = Motor3::from_rotation_translation(Rotor::from_axis_angle(axis, angle), t);
        prop_assert!(close(motor.inverse().transform_point(motor.transform_point(p)), p));
    }

    #[test]
    fn cross_product_is_perpendicular_to_operands(a in vector(), b in vector()) {
        let cross = a.cross(b);
        let scale = 1.0 + a.length() * b.length();
        prop_assert!(cross.dot(a).abs() <= TOLERANCE * scale * (1.0 + a.length()));
        prop_assert!(cross.dot(b).abs() <= TOLERANCE * scale * (1.0 + b.length()));
    }

    #[test]
    fn normalize_yields_unit_length(v in axis()) {
        prop_assert!((v.normalize().length() - 1.0).abs() <= TOLERANCE);
    }
}
