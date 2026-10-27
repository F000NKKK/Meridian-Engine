//! Tests of the float flavor's rotors, motors, bivectors and projections.

use core::f32::consts::PI;

use meridian_numeric_core::Scalar;

use crate::*;

fn assert_vec3_approx(a: Vec3, b: Vec3) {
    assert!(
        a.approx_eq(b),
        "expected {b:?} to approximately equal {a:?}"
    );
}

/// Independent oracle: Rodrigues' rotation formula, computed with
/// plain vector arithmetic (no geometric algebra), to cross-check
/// `Rotor`/`Motor3` against a ground truth that doesn't share any
/// code path with the PGA implementation.
fn rodrigues(v: Vec3, axis: Vec3, angle: Scalar) -> Vec3 {
    let n = axis.normalize();
    v * angle.cos() + n.cross(v) * angle.sin() + n * (n.dot(v) * (1.0 - angle.cos()))
}

#[test]
fn rotor_matches_rodrigues_oracle() {
    let axis = Vec3::new(0.3, -0.7, 1.2);
    let angle = 1.234;
    let v = Vec3::new(1.0, 2.0, -3.0);

    let rotor = Rotor::from_axis_angle(axis, angle);
    let got = rotor.transform_vector(v);
    let want = rodrigues(v, axis, angle);
    assert_vec3_approx(got, want);
}

#[test]
fn rotor_about_z_is_right_handed() {
    let rotor = Rotor::from_axis_angle(Vec3::Z, PI / 2.0);
    let got = rotor.transform_vector(Vec3::X);
    assert_vec3_approx(got, Vec3::Y);
}

#[test]
fn rotor_composition_adds_angles() {
    let axis = Vec3::new(0.2, 0.4, 0.9);
    let a = Rotor::from_axis_angle(axis, 0.5);
    let b = Rotor::from_axis_angle(axis, 0.8);
    let composed = a.compose(b);
    let direct = Rotor::from_axis_angle(axis, 1.3);

    let v = Vec3::new(1.0, 0.5, -0.25);
    assert_vec3_approx(composed.transform_vector(v), direct.transform_vector(v));
}

#[test]
fn translation_is_additive() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(-0.5, 4.0, 0.25);
    let p = Vec3::new(10.0, -5.0, 2.0);

    let composed = Motor3::translation(a).compose(Motor3::translation(b));
    let direct = Motor3::translation(a + b);

    assert_vec3_approx(composed.transform_point(p), direct.transform_point(p));
    assert_vec3_approx(direct.transform_point(p), p + a + b);
}

#[test]
fn motor_matches_rotate_then_translate_oracle() {
    let axis = Vec3::new(-0.4, 0.1, 0.9);
    let angle = 0.9;
    let t = Vec3::new(2.0, -1.0, 0.5);
    let p = Vec3::new(1.0, 0.0, 0.0);

    let motor = Motor3::from_rotation_translation(Rotor::from_axis_angle(axis, angle), t);
    let got = motor.transform_point(p);

    // independent oracle: rotate via Rodrigues, then translate
    let want = rodrigues(p, axis, angle) + t;

    assert_vec3_approx(got, want);
}

#[test]
fn motor_inverse_round_trips() {
    let motor = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(0.3, 0.6, -0.2), 0.77),
        Vec3::new(3.0, -2.0, 1.0),
    );
    let p = Vec3::new(5.0, 6.0, 7.0);

    let round_tripped = motor.inverse().transform_point(motor.transform_point(p));
    assert_vec3_approx(round_tripped, p);
}

/// The roadmap milestone: parent/child transform hierarchy propagation.
/// Transforming a point through the child's local motor and then the
/// parent's motor must equal transforming it through the single
/// composed world motor — this is the whole reason `Transform`
/// composition is "one multiplication" instead of a position-plus-
/// rotation merge (see docs/gac-design.md).
#[test]
fn parent_child_hierarchy_composition() {
    let parent = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Z, PI / 4.0),
        Vec3::new(10.0, 0.0, 0.0),
    );
    let child = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Y, PI / 6.0),
        Vec3::new(0.0, 2.0, 0.0),
    );
    let local_point = Vec3::new(1.0, 0.0, 0.0);

    let world_motor = child.compose(parent);
    let via_composed_motor = world_motor.transform_point(local_point);

    let via_step_by_step = parent.transform_point(child.transform_point(local_point));

    assert_vec3_approx(via_composed_motor, via_step_by_step);
}

#[test]
fn bivector_exp_matches_rotor_from_axis_angle_directly() {
    let axis = Vec3::new(0.2, -0.5, 0.8);
    let angle = 1.1;
    let unit_axis = axis.normalize();
    let bivector = Bivector3::new(unit_axis.x, unit_axis.y, unit_axis.z) * angle;

    let via_exp = bivector.exp();
    let via_direct = Rotor::from_axis_angle(axis, angle);

    let p = Vec3::new(1.0, 2.0, 3.0);
    assert_vec3_approx(via_exp.transform_vector(p), via_direct.transform_vector(p));
}

#[test]
fn bivector_exp_of_zero_is_identity() {
    let rotor = Bivector3::ZERO.exp();
    let p = Vec3::new(1.0, 2.0, 3.0);
    assert_vec3_approx(rotor.transform_vector(p), p);
}

#[test]
fn integrating_constant_angular_velocity_matches_one_big_rotation() {
    // Spinning at a constant angular velocity for `steps` small dt's
    // and composing the per-step rotors must match one direct
    // rotation by the total angle swept — the same "many small
    // compositions = one big one" property `rotor_composition_adds_angles`
    // checks, but exercised through the exponential-map integration
    // path a physics integrator actually uses.
    let angular_velocity = Bivector3::new(0.0, 0.0, 3.0); // "about Z", magnitude 3 rad/s
    let dt = 0.001;
    let steps = 1000;

    let mut accumulated = Rotor::identity();
    for _ in 0..steps {
        let step_rotor = (angular_velocity * dt).exp();
        accumulated = accumulated.compose(step_rotor);
    }

    let direct = Rotor::from_axis_angle(Vec3::Z, angular_velocity.length() * dt * steps as f32);

    // A looser tolerance than assert_vec3_approx's 1e-5 is correct
    // here, not a workaround: this test measures f32 accumulation
    // error over 1000 sequential compositions, which is a real
    // property of the floating-point path, not the algebra being
    // wrong. 1000 steps at 1kHz is already more substeps than a
    // physics integrator runs per frame at 60-120Hz.
    let p = Vec3::X;
    let got = accumulated.transform_vector(p);
    let want = direct.transform_vector(p);
    assert!(
        (got - want).length() < 5e-4,
        "accumulated drift too large: got {got:?}, want {want:?}"
    );
}

#[test]
fn wedge_matches_cross_product_components() {
    let a = Vec3::new(1.0, 0.0, 0.0);
    let b = Vec3::new(0.0, 1.0, 0.0);
    let bivector = Bivector3::wedge(a, b);
    let cross = a.cross(b);
    assert!((bivector.e23 - cross.x).abs() < 1e-6);
    assert!((bivector.e31 - cross.y).abs() < 1e-6);
    assert!((bivector.e12 - cross.z).abs() < 1e-6);
}

fn mat4_mul_point(m: [[Scalar; 4]; 4], p: Vec3) -> Vec3 {
    // Column-major, column-vector convention: v' = M * [p, 1]^T.
    let x = m[0][0] * p.x + m[1][0] * p.y + m[2][0] * p.z + m[3][0];
    let y = m[0][1] * p.x + m[1][1] * p.y + m[2][1] * p.z + m[3][1];
    let z = m[0][2] * p.x + m[1][2] * p.y + m[2][2] * p.z + m[3][2];
    Vec3::new(x, y, z)
}

#[test]
fn to_mat4_reproduces_transform_point_for_translation() {
    let motor = Motor3::translation(Vec3::new(1.0, 2.0, 3.0));
    let p = Vec3::new(5.0, -1.0, 0.5);
    assert_vec3_approx(mat4_mul_point(motor.to_mat4(), p), motor.transform_point(p));
}

#[test]
fn to_mat4_reproduces_transform_point_for_rotation_and_translation() {
    let motor = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Z, PI / 3.0),
        Vec3::new(-2.0, 4.0, 1.0),
    );
    for p in [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(2.0, -3.0, 7.0),
    ] {
        assert_vec3_approx(mat4_mul_point(motor.to_mat4(), p), motor.transform_point(p));
    }
}

#[test]
fn motor_transform_vector_ignores_translation() {
    let motor = Motor3::translation(Vec3::new(100.0, -50.0, 25.0));
    // A pure translation has no rotational part: every direction must
    // come back unchanged.
    for v in [Vec3::X, Vec3::Y, Vec3::new(1.0, 2.0, 3.0)] {
        assert_vec3_approx(motor.transform_vector(v), v);
    }
}

#[test]
fn motor_transform_vector_matches_rotor_for_pure_rotation_plus_translation() {
    let rotor = Rotor::from_axis_angle(Vec3::Z, PI / 3.0);
    let motor = Motor3::from_rotation_translation(rotor, Vec3::new(7.0, -3.0, 2.0));
    let v = Vec3::new(1.0, 2.0, -1.0);
    // Translation must cancel out of transform_vector regardless of
    // how large it is, leaving exactly the rotor's own action.
    assert_vec3_approx(motor.transform_vector(v), rotor.transform_vector(v));
}

#[test]
fn to_mat4_identity_is_the_identity_matrix() {
    let m = Motor3::identity().to_mat4();
    let mut expected = [[0.0; 4]; 4];
    for (i, row) in expected.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    assert_eq!(m, expected);
}
