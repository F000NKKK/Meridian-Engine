//! Round trips of every stored GA type through text, binary and frames.

use meridian_numeric_core::Fixed;
use substrate_data::schema::NamedType;
use substrate_data::{Problem, Schema, Shape, frame, read_bytes, to_bytes};

use super::{MotorBlades, StoredScalar};
use crate::fixed_ga::{FixedBivector3, FixedMotor3, FixedRotor, FixedVec3};
use crate::{Bivector3, Motor3, Rotor, Vec3};

const TRICKY: [f32; 6] = [0.1, 1e-7, -0.0, -3.75, f32::MAX, f32::MIN_POSITIVE];

/// `v` through text, binary and a frame; each frame also readable by shape alone.
fn every_form<T: Schema>(v: &T) -> [T; 3] {
    let text = substrate_data::to_text(v);
    let t = substrate_data::from_text::<T>(&text)
        .unwrap_or_else(|e| panic!("{e}\n{text}"))
        .value;
    let b = read_bytes::<T>(&to_bytes(v)).unwrap();
    let bytes = frame::to_frame(v);
    assert!(frame::read_shaped(&bytes, &T::shape()).is_some(), "{text}");
    [t, b, frame::read_frame::<T>(&bytes).unwrap()]
}

/// Every form of `v` has `v`'s components, bit for bit.
fn assert_exact<T, S, const N: usize>(v: &T, components: fn(&T) -> [S; N])
where
    T: Schema,
    S: StoredScalar,
{
    for back in every_form(v) {
        let (got, want) = (components(&back), components(v));
        assert!(S::all_same_bits(got, want), "{got:?} vs {want:?}");
    }
}

fn text_of<T: Schema>(v: &T) -> String {
    substrate_data::to_text(v).split_whitespace().collect()
}

#[test]
fn vectors_and_bivectors_round_trip_bit_exactly() {
    for w in TRICKY.windows(3) {
        assert_exact(&Vec3::new(w[0], w[1], w[2]), Vec3::to_stored);
        assert_exact(&Bivector3::new(w[2], w[0], w[1]), Bivector3::to_stored);
    }
    let v: Vec3 = substrate_data::from_text("[-0.0, 0, 1.5]").unwrap().value;
    assert_eq!((v.x.to_bits(), v.z), ((-0.0f32).to_bits(), 1.5));
}

#[test]
fn component_order_is_gac_cores() {
    assert_eq!(text_of(&Bivector3::new(1.0, 2.0, 3.0)), "[1.0,2.0,3.0]");
    // Oracle: R = cos(θ/2) - sin(θ/2)(n1 e23 + n2 e31 + n3 e12).
    let theta = 1.2f32;
    let (s, c) = (theta * 0.5).sin_cos();
    assert_eq!(
        Rotor::from_axis_angle(Vec3::Y, theta).to_stored(),
        [c, 0.0, -s, 0.0]
    );
    assert_eq!(
        Rotor::from_axis_angle(Vec3::Z, theta).to_stored(),
        [c, 0.0, 0.0, -s]
    );
    let r = Rotor::from_stored([c, 0.0, -s, 0.0]).unwrap();
    let p = r.transform_vector(Vec3::X);
    assert!(
        (p.x - theta.cos()).abs() < 1e-6 && (p.z + theta.sin()).abs() < 1e-6,
        "{p:?}"
    );
}

#[test]
fn rotors_and_motors_round_trip_bit_exactly() {
    let r = Rotor::from_axis_angle(Vec3::new(0.1, -0.7, 0.3), 2.5)
        .compose(Rotor::from_axis_angle(Vec3::X, 1e-7));
    assert_exact(&r, Rotor::to_stored);
    let translated = Motor3::translation(Vec3::new(0.1, 1.5, -0.0));
    assert!(text_of(&translated).contains("translation"));
    assert_exact(&translated, Motor3::even_components);
    let general = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7),
        Vec3::new(0.1, 1e-7, -12.5),
    )
    .compose(Motor3::rotation(Vec3::Y, 0.3));
    // A translation recovered through a rotation rounds: `dual` is written.
    assert!(text_of(&general).contains("dual"));
    assert_exact(&general, Motor3::even_components);
}

#[test]
fn a_written_motor_reads_as_rotation_then_translation() {
    let text = "{ rotor: [1, 0, 0, 0], translation: [0, 2, 0] } // hand-written";
    let m: Motor3 = substrate_data::from_text(text).unwrap().value;
    assert_eq!(m.transform_point(Vec3::X), Vec3::new(1.0, 2.0, 0.0));
    let only_rotor: Motor3 = substrate_data::from_text("{ rotor: [1, 0, 0, 0] }")
        .unwrap()
        .value;
    assert_eq!(only_rotor, Motor3::identity());
}

#[test]
fn motor_problems_are_typed_and_placed() {
    let both = "{ rotor: [1, 0, 0, 0],\n  translation: [0, 0, 0], dual: [0, 0, 0, 0] }";
    let e = substrate_data::from_text::<Motor3>(both).unwrap_err();
    assert_eq!(e.0[0].code, Some(Problem::Invalid.into()));
    let e = substrate_data::from_text::<Motor3>("{\n  rotr: [1, 0, 0, 0] }").unwrap_err();
    let codes: Vec<_> = e.0.iter().map(|d| (d.code, d.line)).collect();
    assert!(
        codes.contains(&(Some(Problem::UnknownField.into()), Some(2))),
        "{e}"
    );
    assert!(
        codes
            .iter()
            .any(|c| c.0 == Some(Problem::MissingField.into())),
        "{e}"
    );
    let e = substrate_data::from_text::<Motor3>("{ rotor: [1, 0, 0] }").unwrap_err();
    assert_eq!(e.0[0].path.to_string(), "rotor");
}

#[test]
fn fixed_flavours_store_raw_bits() {
    let v = FixedVec3::new(Fixed::ZERO, Fixed::from_num(1.5), Fixed::from_bits(-1));
    assert_eq!(text_of(&v), "[0,98304,-1]");
    assert_exact(&v, FixedVec3::to_stored);
    let b = FixedBivector3::new(Fixed::from_bits(i32::MIN), Fixed::ONE, Fixed::from_bits(7));
    assert_exact(&b, FixedBivector3::to_stored);
    let axis = FixedVec3::new(Fixed::ONE, Fixed::ONE, Fixed::ZERO);
    let r = FixedRotor::from_axis_angle(axis, Fixed::from_num(0.9));
    assert_exact(&r, FixedRotor::to_stored);
    let t = FixedVec3::new(
        Fixed::from_num(3.25),
        Fixed::from_bits(1),
        Fixed::from_num(-2.0),
    );
    let m = FixedMotor3::from_rotation_translation(r, t);
    assert_exact(&m, FixedMotor3::even_components);
}

#[test]
fn every_type_describes_itself_by_name() {
    let name = |s: Shape| match s {
        Shape::Named(n) => n.name(),
        other => panic!("{other:?} isn't named"),
    };
    let names = [
        name(Vec3::shape()),
        name(Bivector3::shape()),
        name(Rotor::shape()),
        name(Motor3::shape()),
        name(FixedVec3::shape()),
        name(FixedBivector3::shape()),
        name(FixedRotor::shape()),
        name(FixedMotor3::shape()),
    ];
    let want = ["vec3", "bivector3", "rotor", "motor"];
    assert_eq!(names[..4], want);
    assert_eq!(names[4..], want.map(|n| ["fixed_", n].concat()));
    let Shape::Named(motor) = Motor3::shape() else {
        unreachable!()
    };
    assert!(matches!(motor.stored(), Shape::Struct(s) if s.fields.len() == 3));
}
