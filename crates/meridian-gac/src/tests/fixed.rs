//! Tests of the fixed-point flavor against the float oracle.

use meridian_numeric::Fixed;

use crate::*;

fn fv3(x: f64, y: f64, z: f64) -> FixedVec3 {
    FixedVec3::new(Fixed::from_num(x), Fixed::from_num(y), Fixed::from_num(z))
}

fn assert_fixed_vec3_approx(got: FixedVec3, want: Vec3) {
    let tolerance = 1e-3; // CORDIC/multiply rounding through several ops.
    assert!(
        (got.x.to_num() - want.x as f64).abs() < tolerance
            && (got.y.to_num() - want.y as f64).abs() < tolerance
            && (got.z.to_num() - want.z as f64).abs() < tolerance,
        "expected ~{want:?}, got ({}, {}, {})",
        got.x.to_num(),
        got.y.to_num(),
        got.z.to_num()
    );
}

#[test]
fn fixed_translation_matches_float_ga_oracle() {
    let t = Vec3::new(1.0, 2.0, 3.0);
    let p = Vec3::new(5.0, -1.0, 0.5);
    let want = Motor3::translation(t).transform_point(p);

    let ft = fv3(1.0, 2.0, 3.0);
    let fp = fv3(5.0, -1.0, 0.5);
    let got = FixedMotor3::translation(ft).transform_point(fp);

    assert_fixed_vec3_approx(got, want);
}

#[test]
fn fixed_rotation_matches_float_ga_rodrigues_oracle() {
    let angle = 1.234_f32;
    let axis = Vec3::new(0.0, 0.0, 1.0);
    let p = Vec3::new(1.0, 0.0, 0.0);
    let want = Rotor::from_axis_angle(axis, angle).transform_vector(p);

    let fangle = Fixed::from_num(angle as f64);
    let faxis = fv3(0.0, 0.0, 1.0);
    let fp = fv3(1.0, 0.0, 0.0);
    let got = FixedRotor::from_axis_angle(faxis, fangle).transform_vector(fp);

    assert_fixed_vec3_approx(got, want);
}

#[test]
fn fixed_rotation_translation_composition_matches_float_ga_oracle() {
    let rotor = Rotor::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.7);
    let t = Vec3::new(3.0, -2.0, 1.0);
    let p = Vec3::new(2.0, 1.0, 0.0);
    let want = Motor3::from_rotation_translation(rotor, t).transform_point(p);

    let frotor = FixedRotor::from_axis_angle(fv3(0.0, 0.0, 1.0), Fixed::from_num(0.7));
    let ft = fv3(3.0, -2.0, 1.0);
    let fp = fv3(2.0, 1.0, 0.0);
    let got = FixedMotor3::from_rotation_translation(frotor, ft).transform_point(fp);

    assert_fixed_vec3_approx(got, want);
}

#[test]
fn fixed_motor_inverse_round_trips() {
    let rotor = FixedRotor::from_axis_angle(fv3(0.3, 0.6, 0.2), Fixed::from_num(0.9));
    let motor = FixedMotor3::from_rotation_translation(rotor, fv3(4.0, -1.0, 2.0));
    let p = fv3(1.0, 2.0, 3.0);

    let round_tripped = motor.inverse().transform_point(motor.transform_point(p));

    let tolerance = Fixed::from_num(1e-3);
    assert!((round_tripped.x - p.x).abs() < tolerance);
    assert!((round_tripped.y - p.y).abs() < tolerance);
    assert!((round_tripped.z - p.z).abs() < tolerance);
}

#[test]
fn fixed_bivector_exp_matches_rotor_from_axis_angle_directly() {
    let bivector = FixedBivector3::new(Fixed::ZERO, Fixed::ZERO, Fixed::from_num(0.5));
    let via_exp = bivector.exp();
    let direct = FixedRotor::from_axis_angle(fv3(0.0, 0.0, 1.0), Fixed::from_num(0.5));

    let p = fv3(1.0, 0.0, 0.0);
    let a = via_exp.transform_vector(p);
    let b = direct.transform_vector(p);
    let tolerance = Fixed::from_num(1e-3);
    assert!((a.x - b.x).abs() < tolerance);
    assert!((a.y - b.y).abs() < tolerance);
    assert!((a.z - b.z).abs() < tolerance);
}

#[test]
fn fixed_aabb_support_matches_float_aabb_support() {
    let faabb = FixedAabb {
        min: fv3(-1.0, -2.0, -3.0),
        max: fv3(4.0, 5.0, 6.0),
    };
    let aabb = Aabb {
        min: Vec3::new(-1.0, -2.0, -3.0),
        max: Vec3::new(4.0, 5.0, 6.0),
    };
    for direction in [fv3(1.0, 1.0, 1.0), fv3(-1.0, -1.0, -1.0)] {
        let got = faabb.support(direction).to_float_lossy();
        let want = aabb.support(direction.to_float_lossy());
        assert!((got.x - want.x).abs() < 1e-3);
        assert!((got.y - want.y).abs() < 1e-3);
        assert!((got.z - want.z).abs() < 1e-3);
    }
}

#[test]
fn fixed_obb_support_matches_float_obb_support() {
    let frame = FixedRotor::from_axis_angle(fv3(0.0, 0.0, 1.0), Fixed::pi() / Fixed::from_num(2.0));
    let fobb = FixedObb {
        frame: FixedMotor3::from_rotation_translation(frame, FixedVec3::ZERO),
        half_extents: fv3(3.0, 1.0, 1.0),
    };
    let got = fobb.support(fv3(0.0, 1.0, 0.0)).to_float_lossy();
    assert!((got.y - 3.0).abs() < 1e-2);
}

#[test]
fn fixed_convex_volume_generalizes_intersects_to_any_shape() {
    let one = Fixed::ONE;
    let volume = FixedConvexVolume::new(vec![
        FixedPlane {
            normal: fv3(1.0, 0.0, 0.0),
            d: one,
        },
        FixedPlane {
            normal: fv3(-1.0, 0.0, 0.0),
            d: one,
        },
        FixedPlane {
            normal: fv3(0.0, 1.0, 0.0),
            d: one,
        },
        FixedPlane {
            normal: fv3(0.0, -1.0, 0.0),
            d: one,
        },
        FixedPlane {
            normal: fv3(0.0, 0.0, 1.0),
            d: one,
        },
        FixedPlane {
            normal: fv3(0.0, 0.0, -1.0),
            d: one,
        },
    ]);

    assert!(volume.intersects(&FixedSphere {
        center: FixedVec3::ZERO,
        radius: Fixed::from_num(0.5),
    }));
    assert!(!volume.intersects(&FixedSphere {
        center: fv3(10.0, 0.0, 0.0),
        radius: Fixed::from_num(0.5),
    }));
}

#[test]
fn lossy_round_trip_preserves_value_within_precision() {
    let v = Vec3::new(1.5, -2.25, 3.75);
    let round_tripped = v.to_fixed_lossy().to_float_lossy();
    assert!((round_tripped.x - v.x).abs() < 1e-3);
    assert!((round_tripped.y - v.y).abs() < 1e-3);
    assert!((round_tripped.z - v.z).abs() < 1e-3);
}

#[test]
fn cross_type_add_matches_converting_first() {
    let a = fv3(1.0, 2.0, 3.0);
    let b = Vec3::new(0.5, -1.0, 2.0);

    let mixed = a + b;
    let converted_first = a + b.to_fixed_lossy();
    assert_eq!(mixed, converted_first);

    let mixed_float = b + a;
    let converted_first_float = b + a.to_float_lossy();
    assert!((mixed_float.x - converted_first_float.x).abs() < 1e-3);
}

#[test]
fn motor_to_fixed_lossy_matches_transform_point_within_precision() {
    let motor = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.6),
        Vec3::new(2.0, -1.0, 3.0),
    );
    let p = Vec3::new(1.0, 1.0, 1.0);
    let want = motor.transform_point(p);
    let got = motor.to_fixed_lossy().transform_point(p.to_fixed_lossy());
    assert!((got.x.to_num() as f32 - want.x).abs() < 1e-2);
    assert!((got.y.to_num() as f32 - want.y).abs() < 1e-2);
    assert!((got.z.to_num() as f32 - want.z).abs() < 1e-2);
}
