//! The hand-expanded `Motor3::transform_point` against the full sandwich
//! product `M P M~` it replaces.

use meridian_gac_core::multivector::float::Multivector;
use meridian_gac_core::{Motor3, Rotor, Vec3, blade};

fn sandwich(motor: Motor3, point: Vec3) -> Vec3 {
    let mut pm = [0.0; 16];
    pm[blade::E123] = 1.0;
    pm[blade::E023] = -point.x;
    pm[blade::E013] = point.y;
    pm[blade::E012] = -point.z;
    let r = motor.0 * Multivector(pm) * motor.0.reverse();
    Vec3::new(-r.0[blade::E023], r.0[blade::E013], -r.0[blade::E012])
}

fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-4
}

#[test]
fn transform_point_matches_the_full_sandwich_product() {
    let cases = [
        (Vec3::new(1.0, 2.0, 3.0), 0.4, 0.0),
        (Vec3::new(-0.3, 0.9, 0.2), 1.3, 0.25),
        (Vec3::Y, 2.2, 0.5),
    ];
    let points = [
        Vec3::ZERO,
        Vec3::X,
        Vec3::new(0.3, -1.2, 4.5),
        Vec3::new(-7.0, 2.0, 0.5),
    ];
    for (axis, angle, z) in cases {
        let motor = Motor3::from_rotation_translation(
            Rotor::from_axis_angle(axis, angle),
            Vec3::new(1.5, -2.0, z),
        );
        for point in points {
            assert!(close(motor.transform_point(point), sandwich(motor, point)));
        }
    }
}

#[test]
fn composed_motors_keep_every_blade_consistent() {
    let a = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.3),
        Vec3::new(1.0, 2.0, 3.0),
    );
    let b = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(1.0, 1.0, 0.0), -1.1),
        Vec3::new(-4.0, 0.5, 2.0),
    );
    let both = a.compose(b);
    let p = Vec3::new(0.7, -0.2, 1.9);
    assert!(close(both.transform_point(p), sandwich(both, p)));
}

#[test]
fn transform_vector_drops_the_translation() {
    let motor = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7),
        Vec3::new(9.0, -8.0, 7.0),
    );
    let v = Vec3::new(0.3, -1.2, 4.5);
    assert!(close(
        motor.transform_vector(v),
        sandwich(motor, v) - sandwich(motor, Vec3::ZERO)
    ));
}

#[test]
fn translation_only_motors_and_inverses_match() {
    let motor = Motor3::translation(Vec3::new(2.0, 0.0, -1.0));
    for m in [motor, motor.inverse()] {
        for p in [Vec3::ZERO, Vec3::X, Vec3::new(1.0, 2.0, 3.0)] {
            assert!(close(m.transform_point(p), sandwich(m, p)), "{m:?} {p:?}");
        }
    }
}
