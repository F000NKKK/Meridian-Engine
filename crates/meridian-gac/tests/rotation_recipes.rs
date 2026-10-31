//! Executable recipes for application-order composition and checked rotations.
use meridian_gac::{Motor3, RotationError, Rotor, Vec3};
use std::f32::consts::FRAC_PI_2;

fn close(actual: Vec3, expected: Vec3) {
    assert!(
        (actual - expected).length() < 2e-5,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn positive_rotation_and_inverse_follow_the_right_hand_rule() {
    let rotation = Rotor::try_from_axis_angle(Vec3::Z, FRAC_PI_2).unwrap();
    close(rotation.transform_vector(Vec3::X), Vec3::Y);
    close(rotation.reverse().transform_vector(Vec3::Y), Vec3::X);
}

#[test]
fn rotation_then_translation_differs_from_local_motion() {
    let rotation = Rotor::try_from_axis_angle(Vec3::Z, FRAC_PI_2).unwrap();
    let pose = Motor3::from_rotation_translation(rotation, Vec3::X);
    close(pose.transform_point(Vec3::X), Vec3::X + Vec3::Y);
    close(pose.transform_vector(Vec3::X), Vec3::Y);
    let local_step = Motor3::translation(Vec3::X).compose(pose);
    close(local_step.transform_point(Vec3::ZERO), Vec3::X + Vec3::Y);
    let world_step = pose.compose(Motor3::translation(Vec3::X));
    close(world_step.transform_point(Vec3::ZERO), Vec3::X * 2.0);
}

#[test]
fn checked_axes_reject_invalid_inputs_and_accept_extreme_finite_scales() {
    assert_eq!(
        Rotor::try_from_axis_angle(Vec3::ZERO, 1.0),
        Err(RotationError::ZeroDirection)
    );
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            Rotor::try_from_axis_angle(Vec3::X, bad),
            Err(RotationError::NonFinite)
        );
        assert_eq!(
            Rotor::try_from_axis_angle(Vec3::new(bad, 0.0, 0.0), 1.0),
            Err(RotationError::NonFinite)
        );
    }
    for scale in [f32::MAX, f32::MIN_POSITIVE, f32::from_bits(1)] {
        let rotation = Rotor::try_from_axis_angle(Vec3::Z * scale, FRAC_PI_2).unwrap();
        close(rotation.transform_vector(Vec3::X), Vec3::Y);
    }
}

#[test]
fn direction_alignment_handles_parallel_opposite_and_oblique_inputs() {
    for target in [Vec3::X, -Vec3::X, Vec3::Y, Vec3::new(1.0, 2.0, 3.0)] {
        let rotor = Rotor::between_directions(Vec3::X, target).unwrap();
        close(rotor.transform_vector(Vec3::X), target.normalize());
    }
    assert_eq!(
        Rotor::between_directions(Vec3::ZERO, Vec3::X),
        Err(RotationError::ZeroDirection)
    );
    assert_eq!(
        Rotor::between_directions(Vec3::X, Vec3::ZERO),
        Err(RotationError::ZeroDirection)
    );
    assert_eq!(
        Rotor::between_directions(Vec3::X, Vec3::new(0.0, f32::NAN, 0.0)),
        Err(RotationError::NonFinite)
    );
}

#[test]
fn composition_applies_left_operand_first() {
    let first = Rotor::from_axis_angle(Vec3::Z, FRAC_PI_2);
    let second = Rotor::from_axis_angle(Vec3::X, FRAC_PI_2);
    close(first.compose(second).transform_vector(Vec3::X), Vec3::Z);
    close(second.compose(first).transform_vector(Vec3::X), Vec3::Y);
}
