//! Tests of the float rotor's quaternion interchange and `slerp`.

use core::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use crate::*;

fn assert_vec3_approx(a: Vec3, b: Vec3) {
    assert!(
        a.approx_eq(b),
        "expected {b:?} to approximately equal {a:?}"
    );
}

#[test]
fn a_quarter_turn_quaternion_about_z_sweeps_x_to_y() {
    let half = FRAC_PI_4;
    let rotor = Rotor::from_quaternion([0.0, 0.0, half.sin(), half.cos()]);
    assert_vec3_approx(rotor.transform_vector(Vec3::X), Vec3::Y);
}

#[test]
fn quaternions_and_axis_angles_describe_the_same_rotation() {
    let axis = Vec3::new(0.3, -0.7, 1.2).normalize();
    let angle = 1.234f32;
    let s = (angle / 2.0).sin();
    let from_quaternion =
        Rotor::from_quaternion([axis.x * s, axis.y * s, axis.z * s, (angle / 2.0).cos()]);
    let from_axis = Rotor::from_axis_angle(axis, angle);
    let v = Vec3::new(1.0, 2.0, -3.0);
    assert_vec3_approx(
        from_quaternion.transform_vector(v),
        from_axis.transform_vector(v),
    );
}

#[test]
fn to_quaternion_inverts_from_quaternion() {
    let q = [0.1, -0.5, 0.3, 0.8];
    let length = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    let back = Rotor::from_quaternion(q).to_quaternion();
    for (got, want) in back.iter().zip(q) {
        assert!((got - want / length).abs() < 1e-6, "{back:?}");
    }
}

#[test]
fn a_zero_quaternion_is_the_identity() {
    assert_eq!(Rotor::from_quaternion([0.0; 4]), Rotor::identity());
}

#[test]
fn slerp_halfway_between_two_turns_about_z_is_the_turn_between_them() {
    let a = Rotor::from_axis_angle(Vec3::Z, 0.0);
    let b = Rotor::from_axis_angle(Vec3::Z, FRAC_PI_2);
    let mid = a.slerp(b, 0.5);
    assert_vec3_approx(
        mid.transform_vector(Vec3::X),
        Rotor::from_axis_angle(Vec3::Z, FRAC_PI_4).transform_vector(Vec3::X),
    );
}

#[test]
fn slerp_hits_both_ends_and_takes_the_short_way() {
    let a = Rotor::from_axis_angle(Vec3::Y, 0.4);
    let b = Rotor::from_axis_angle(Vec3::Y, 1.0);
    let v = Vec3::new(1.0, 0.0, 2.0);
    assert_vec3_approx(a.slerp(b, 0.0).transform_vector(v), a.transform_vector(v));
    assert_vec3_approx(a.slerp(b, 1.0).transform_vector(v), b.transform_vector(v));
    let negated = Rotor::from_quaternion(b.to_quaternion().map(|c| -c));
    assert_vec3_approx(
        a.slerp(negated, 0.5).transform_vector(v),
        a.slerp(b, 0.5).transform_vector(v),
    );
}
