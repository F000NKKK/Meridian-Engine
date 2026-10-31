//! Orientation and validation regression tests.
use meridian_gac::{Multivector, RotationError, Rotor, Vec3, blade};
use proptest::prelude::*;

fn close(actual: Vec3, expected: Vec3) {
    assert!(
        (actual - expected).length() < 3e-5,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn look_rotation_controls_forward_and_roll_including_reverse_forward() {
    for forward in [Vec3::Z, -Vec3::Z, Vec3::X, Vec3::new(1.0, 2.0, 3.0)] {
        for up in [Vec3::Y, -Vec3::Y, Vec3::new(2.0, 3.0, 1.0)] {
            let rotor = Rotor::look_rotation(forward, up).unwrap();
            let direction = forward.normalize();
            let expected_up = direction.cross(up.cross(direction).normalize());
            close(rotor.transform_vector(Vec3::Z), direction);
            close(rotor.transform_vector(Vec3::Y), expected_up);
        }
    }
}

#[test]
fn undefined_look_inputs_return_typed_errors() {
    for up in [Vec3::Z, -Vec3::Z, Vec3::new(1e-8, 0.0, 1.0)] {
        assert_eq!(
            Rotor::look_rotation(Vec3::Z, up),
            Err(RotationError::ParallelUp)
        );
    }
    assert_eq!(
        Rotor::look_rotation(Vec3::ZERO, Vec3::Y),
        Err(RotationError::ZeroDirection)
    );
    assert_eq!(
        Rotor::look_rotation(Vec3::Z, Vec3::ZERO),
        Err(RotationError::ZeroDirection)
    );
    assert_eq!(
        Rotor::look_rotation(Vec3::Z, Vec3::new(f32::INFINITY, 0.0, 0.0)),
        Err(RotationError::NonFinite)
    );
}

#[test]
fn raw_validation_normalizes_without_overflow_or_underflow() {
    for scalar in [f32::MAX, f32::MIN_POSITIVE, f32::from_bits(1), -10.0] {
        let mut coefficients = [0.0; 16];
        coefficients[blade::S] = scalar;
        let rotor = Rotor(Multivector(coefficients)).try_normalized().unwrap();
        close(rotor.transform_vector(Vec3::X), Vec3::X);
    }
    assert_eq!(
        Rotor(Multivector([0.0; 16])).try_normalized(),
        Err(RotationError::InvalidRotor)
    );
    for index in 0..16 {
        let mut coefficients = [0.0; 16];
        coefficients[blade::S] = 1.0;
        coefficients[index] = f32::NAN;
        assert_eq!(
            Rotor(Multivector(coefficients)).try_normalized(),
            Err(RotationError::NonFinite)
        );
        coefficients[index] = 1.0;
        if ![blade::S, blade::E12, blade::E13, blade::E23].contains(&index) {
            assert_eq!(
                Rotor(Multivector(coefficients)).try_normalized(),
                Err(RotationError::InvalidRotor)
            );
        }
    }
}

#[test]
fn checked_interpolation_validates_endpoints_and_fraction() {
    let start = Rotor::identity();
    let end = Rotor::from_axis_angle(Vec3::Z, std::f32::consts::FRAC_PI_2);
    close(
        start.try_slerp(end, 0.0).unwrap().transform_vector(Vec3::X),
        Vec3::X,
    );
    close(
        start.try_slerp(end, 1.0).unwrap().transform_vector(Vec3::X),
        Vec3::Y,
    );
    let half = start.try_slerp(end, 0.5).unwrap();
    close(
        half.transform_vector(Vec3::X),
        Vec3::new(1.0, 1.0, 0.0).normalize(),
    );
    for fraction in [-0.1, 1.1] {
        assert_eq!(
            start.try_slerp(end, fraction),
            Err(RotationError::InvalidFraction)
        );
    }
    assert_eq!(
        start.try_slerp(end, f32::NAN),
        Err(RotationError::NonFinite)
    );
    let invalid = Rotor(Multivector([0.0; 16]));
    assert_eq!(
        invalid.try_slerp(end, 0.5),
        Err(RotationError::InvalidRotor)
    );
    assert_eq!(
        start.try_slerp(invalid, 0.5),
        Err(RotationError::InvalidRotor)
    );
}

proptest! {
    #[test]
    fn look_basis_remains_orthonormal(x in -10f32..10.0, y in -10f32..10.0, z in -10f32..10.0) {
        let forward = Vec3::new(x, y, z);
        prop_assume!(forward.length() > 0.1);
        prop_assume!(forward.normalize().cross(Vec3::Y).length() > 0.01);
        let rotor = Rotor::look_rotation(forward, Vec3::Y).unwrap();
        close(rotor.transform_vector(Vec3::Z), forward.normalize());
        let right = rotor.transform_vector(Vec3::X);
        let up = rotor.transform_vector(Vec3::Y);
        close(right.cross(up), rotor.transform_vector(Vec3::Z));
    }
}
