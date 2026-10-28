//! The hand-expanded `Motor3::transform_point` against the full sandwich
//! product `M P M~` it replaces.

use meridian_gac::multivector::float::Multivector;
use meridian_gac::{Motor3, Rotor, Vec3, blade};

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

mod fixed {
    use meridian_gac::multivector::fixed::FixedMultivector;
    use meridian_gac::{FixedMotor3, FixedRotor, FixedVec3, blade};
    use meridian_numeric::Fixed;

    fn fx(value: f64) -> Fixed {
        Fixed::from_num(value)
    }

    fn vec3(x: f64, y: f64, z: f64) -> FixedVec3 {
        FixedVec3::new(fx(x), fx(y), fx(z))
    }

    fn sandwich(motor: FixedMotor3, point: FixedVec3) -> FixedVec3 {
        let mut pm = [Fixed::ZERO; 16];
        pm[blade::E123] = Fixed::ONE;
        pm[blade::E023] = -point.x;
        pm[blade::E013] = point.y;
        pm[blade::E012] = -point.z;
        let r = motor.0 * FixedMultivector(pm) * motor.0.reverse();
        FixedVec3::new(-r.0[blade::E023], r.0[blade::E013], -r.0[blade::E012])
    }

    /// Within a few units of the last place of Q16.16.
    fn close(a: FixedVec3, b: FixedVec3) -> bool {
        let tolerance = 16;
        [(a.x, b.x), (a.y, b.y), (a.z, b.z)]
            .iter()
            .all(|(l, r)| (l.to_bits() - r.to_bits()).abs() <= tolerance)
    }

    fn motors() -> Vec<FixedMotor3> {
        let rotations = [
            (vec3(1.0, 2.0, 3.0), 0.4),
            (vec3(-0.3, 0.9, 0.2), 1.3),
            (vec3(0.0, 1.0, 0.0), 2.2),
        ];
        rotations
            .iter()
            .map(|&(axis, angle)| {
                FixedMotor3::from_rotation_translation(
                    FixedRotor::from_axis_angle(axis, fx(angle)),
                    vec3(1.5, -2.0, 0.25),
                )
            })
            .collect()
    }

    #[test]
    fn transform_point_matches_the_full_sandwich_product() {
        let points = [
            FixedVec3::ZERO,
            vec3(1.0, 0.0, 0.0),
            vec3(0.3, -1.2, 4.5),
            vec3(-7.0, 2.0, 0.5),
        ];
        for motor in motors() {
            for point in points {
                assert!(close(motor.transform_point(point), sandwich(motor, point)));
            }
        }
    }

    #[test]
    fn transform_vector_drops_the_translation() {
        let v = vec3(0.3, -1.2, 4.5);
        for motor in motors() {
            let expected = sandwich(motor, v) - sandwich(motor, FixedVec3::ZERO);
            assert!(close(motor.transform_vector(v), expected));
        }
    }

    #[test]
    fn composition_and_inverse_still_agree() {
        let [a, b, _] = <[FixedMotor3; 3]>::try_from(motors()).expect("three motors");
        let both = a.compose(b);
        let p = vec3(0.7, -0.2, 1.9);
        assert!(close(both.transform_point(p), sandwich(both, p)));
        let back = both.inverse().transform_point(both.transform_point(p));
        // Q16.16 truncation accumulates over two composed motors; the full
        // product drifts by a comparable amount (a few tens of units).
        let drift = [(back.x, p.x), (back.y, p.y), (back.z, p.z)]
            .iter()
            .map(|(l, r)| (l.to_bits() - r.to_bits()).abs())
            .max()
            .expect("three axes");
        assert!(drift <= 64, "round trip drifted {drift} units");
    }
}

#[test]
fn stretched_scales_the_translation_and_keeps_the_rotation() {
    let turn = Rotor::from_axis_angle(Vec3::Z, 0.7);
    let motor = Motor3::from_rotation_translation(turn, Vec3::new(1.0, 2.0, 3.0));
    let stretched = motor.stretched(Vec3::new(2.0, 3.0, 4.0));

    let origin = stretched.transform_point(Vec3::ZERO);
    for (got, want) in [(origin.x, 2.0), (origin.y, 6.0), (origin.z, 12.0)] {
        assert!((got - want).abs() < 1e-5, "{got} vs {want}");
    }
    let before = motor.transform_vector(Vec3::X);
    let after = stretched.transform_vector(Vec3::X);
    assert!((before - after).length() < 1e-5);
}

#[test]
fn stretching_by_one_changes_nothing() {
    let motor = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Y, 1.1),
        Vec3::new(4.0, 5.0, 6.0),
    );
    assert_eq!(motor.stretched(Vec3::ONE), motor);
}
