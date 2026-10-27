//! Substrate Data schemas for the GA value types (`data` feature) — see
//! docs/data-formats.md ("Math values") and
//! [ADR 016](../../../../docs/adr/016-data-formats-on-substrate-data.md).
//!
//! Every type describes itself as `Shape::Named` (`"vec3"`, `"rotor"`,
//! `"motor"`, `"fixed_vec3"`, …) so an editor that knows the name shows a
//! gizmo and one that doesn't edits the plain components. The logic is
//! written **once**, here, generic over the component scalar ([`DataScalar`]:
//! `f32` or `Fixed`); [`float`] and [`fixed`] only say how each concrete
//! type maps to its components ([`Stored`]) — the same lib/float/fixed
//! split `gac-core` itself uses (CLAUDE.md's "Float/Fixed branching" rule).
//! The float and fixed *types* are separate (see [`crate::fixed_ga`]'s
//! module doc for why), so `impl Schema` itself is stamped per type by
//! [`stored_schema!`] — a blanket impl over a local trait isn't allowed for
//! a foreign trait (orphan rule).
//!
//! Stored forms, in `gac-core`'s own conventions:
//!
//! - `Vec3`: `[x, y, z]`;
//! - `Bivector3`: `[e23, e31, e12]` — [`crate::Bivector3::new`]'s order,
//!   the Hodge duals of the x, y, z axes;
//! - `Rotor`: `[s, e23, e31, e12]` — the even Euclidean blades, bivector
//!   part in `Bivector3`'s order. The multivector keeps `e13`
//!   (canonical increasing order), so `e31` is written as `-e13`; negation
//!   is exact, so this stays bit-exact. A rotor's other twelve blades are
//!   zero by construction and are not stored;
//! - `Motor3`: `{ rotor, translation }` — `translation` is the point the
//!   origin moves to — when rebuilding the motor from those two reproduces
//!   it bit-exactly, otherwise `{ rotor, dual }` with `dual` the motor's
//!   `e0` part `[e01, e02, e03, e0123]` as stored. Reading accepts either
//!   (`translation` is what a person writes). A motor's odd blades are zero
//!   by construction and are not stored.
//!
//! Floats are `f32` carried exactly (text writes the `f64` of the `f32`, so
//! reading it back gives the same bits, `-0.0` included); `Fixed`
//! components are raw Q16.16 bits (`meridian-numeric-core`'s schema).
//! In every form a type is stored as its `Shape::Named`'s `of` (Substrate
//! Data's rule, so `frame::read_shaped` reads it by shape alone): frames
//! carry the component lists packed (`f32` raw, `Fixed` as bits) and a
//! motor as a `{ rotor, translation?, dual? }` record — never as a tagged
//! binary blob.

mod fixed;
mod float;

use substrate_data::frame::{Packer, Unpacker};
use substrate_data::schema::{Field, StructShape};
use substrate_data::{Map, Reader, Schema, Shape, Value, ValueRef};

use crate::blade;
use crate::generic::{MotorLike, ScalarLike, VectorLike};

/// A component scalar of a stored GA value: `f32` or `Fixed`.
pub(crate) trait DataScalar: ScalarLike + Schema {
    /// Whether `self` and `other` are the same bits (`-0.0` is not `0.0`).
    fn same_bits(self, other: Self) -> bool;
}

/// A GA value stored as `N` scalars in a fixed order.
pub(crate) trait Stored<const N: usize>: Copy {
    type S: DataScalar;
    /// What tools know the type by (`Shape::Named`'s `name`).
    const NAME: &'static str;
    fn parts(&self) -> [Self::S; N];
    fn from_parts(parts: [Self::S; N]) -> Self;
}

/// The stored rotor parts `[s, e23, e31, e12]` of an even multivector.
pub(crate) fn rotor_parts<S: ScalarLike>(m: &[S; 16]) -> [S; 4] {
    [m[blade::S], m[blade::E23], -m[blade::E13], m[blade::E12]]
}

/// The motor parts `[s, e23, e31, e12, e01, e02, e03, e0123]`.
pub(crate) fn motor_parts<S: ScalarLike>(m: &[S; 16]) -> [S; 8] {
    let [s, e23, e31, e12] = rotor_parts(m);
    [
        s,
        e23,
        e31,
        e12,
        m[blade::E01],
        m[blade::E02],
        m[blade::E03],
        m[blade::E0123],
    ]
}

/// The multivector of stored rotor parts (other blades zero).
pub(crate) fn rotor_multivector<S: ScalarLike>([s, e23, e31, e12]: [S; 4]) -> [S; 16] {
    let mut m = [S::ZERO; 16];
    m[blade::S] = s;
    m[blade::E23] = e23;
    m[blade::E13] = -e31;
    m[blade::E12] = e12;
    m
}

/// The multivector of stored motor parts (odd blades zero).
pub(crate) fn motor_multivector<S: ScalarLike>(p: [S; 8]) -> [S; 16] {
    let mut m = rotor_multivector([p[0], p[1], p[2], p[3]]);
    m[blade::E01] = p[4];
    m[blade::E02] = p[5];
    m[blade::E03] = p[6];
    m[blade::E0123] = p[7];
    m
}

fn same_bits<S: DataScalar, const N: usize>(a: [S; N], b: [S; N]) -> bool {
    a.into_iter().zip(b).all(|(a, b)| a.same_bits(b))
}

/// `impl Schema` for a [`Stored`] type: `Shape::Named` over its parts as a
/// list, read/written/encoded/packed as that list.
macro_rules! stored_schema {
    ($($t:ty => $n:literal),* $(,)?) => {$(
        impl substrate_data::Schema for $t {
            fn shape() -> substrate_data::Shape {
                substrate_data::Shape::Named {
                    name: <$t as $crate::data::Stored<$n>>::NAME,
                    of: <[<$t as $crate::data::Stored<$n>>::S; $n] as substrate_data::Schema>::shape,
                }
            }
            fn read(
                v: &substrate_data::ValueRef<'_>,
                cx: &mut substrate_data::Reader,
            ) -> Option<Self> {
                <[<$t as $crate::data::Stored<$n>>::S; $n] as substrate_data::Schema>::read(v, cx)
                    .map(<$t as $crate::data::Stored<$n>>::from_parts)
            }
            fn write(&self) -> substrate_data::Value {
                substrate_data::Schema::write(&$crate::data::Stored::<$n>::parts(self))
            }
            fn encode(&self, w: &mut substrate_data::binary::Writer) {
                substrate_data::Schema::encode(&$crate::data::Stored::<$n>::parts(self), w);
            }
            fn decode(d: &mut substrate_data::binary::Decoder<'_>) -> Option<Self> {
                <[<$t as $crate::data::Stored<$n>>::S; $n] as substrate_data::Schema>::decode(d)
                    .map(<$t as $crate::data::Stored<$n>>::from_parts)
            }
            fn pack(&self, p: &mut substrate_data::frame::Packer) {
                substrate_data::Schema::pack(&$crate::data::Stored::<$n>::parts(self), p);
            }
            fn unpack(u: &mut substrate_data::frame::Unpacker<'_>) -> Option<Self> {
                <[<$t as $crate::data::Stored<$n>>::S; $n] as substrate_data::Schema>::unpack(u)
                    .map(<$t as $crate::data::Stored<$n>>::from_parts)
            }
        }
    )*};
}
pub(crate) use stored_schema;

// ── Motors ──────────────────────────────────────────────────────────────────

const ROTOR: &str = "rotor";
const TRANSLATION: &str = "translation";
const DUAL: &str = "dual";

/// What a motor needs to be stored: its eight even parts, its rotor's and
/// its translation vector's.
pub(crate) trait StoredMotor:
    MotorLike<Scalar = <Self as Stored<8>>::S> + Stored<8>
where
    Self::Rotor: Stored<4, S = Self::Scalar>,
    Self::Vector: Stored<3, S = Self::Scalar>,
{
}

impl<M> StoredMotor for M
where
    M: MotorLike<Scalar = <M as Stored<8>>::S> + Stored<8>,
    M::Rotor: Stored<4, S = M::Scalar>,
    M::Vector: Stored<3, S = M::Scalar>,
{
}

/// A motor's shape: `Shape::Named` over `{ rotor, translation?, dual? }`.
pub(crate) fn motor_shape<M>() -> Shape
where
    M: StoredMotor,
    M::Rotor: Stored<4, S = M::Scalar>,
    M::Vector: Stored<3, S = M::Scalar>,
{
    Shape::Named {
        name: <M as Stored<8>>::NAME,
        of: motor_struct_shape::<M>,
    }
}

fn motor_struct_shape<M>() -> Shape
where
    M: StoredMotor,
    M::Rotor: Stored<4, S = M::Scalar>,
    M::Vector: Stored<3, S = M::Scalar>,
{
    let field = |name, doc, shape, required| Field {
        name,
        doc,
        shape,
        required,
        range: None,
    };
    Shape::Struct(StructShape {
        name: <M as Stored<8>>::NAME,
        doc: "A rigid motion: rotate by `rotor`, then move by `translation`.",
        fields: vec![
            field(
                ROTOR,
                "The rotation: [s, e23, e31, e12].",
                <[M::Scalar; 4]>::shape,
                true,
            ),
            field(
                TRANSLATION,
                "Where the origin goes, applied after the rotation: [x, y, z]. Default [0, 0, 0].",
                <Option<[M::Scalar; 3]>>::shape,
                false,
            ),
            field(
                DUAL,
                "The motor's e0 part [e01, e02, e03, e0123] as stored — written instead of \
                 `translation` when that wouldn't rebuild the motor bit-exactly.",
                <Option<[M::Scalar; 4]>>::shape,
                false,
            ),
        ],
    })
}

/// A motor as stored: its rotor parts, and its translation or its `e0`
/// part (see the module doc).
type MotorForm<S> = ([S; 4], Option<[S; 3]>, Option<[S; 4]>);

/// `{ rotor, translation }` when that rebuilds `m` bit-exactly,
/// `{ rotor, dual }` otherwise.
fn motor_form<M>(m: &M) -> MotorForm<M::Scalar>
where
    M: StoredMotor,
    M::Rotor: Stored<4, S = M::Scalar>,
    M::Vector: Stored<3, S = M::Scalar>,
{
    let parts = m.parts();
    let rotor = [parts[0], parts[1], parts[2], parts[3]];
    let t = m.transform_point(<M::Vector as VectorLike>::ZERO);
    let rebuilt = M::from_rotation_translation(<M::Rotor as Stored<4>>::from_parts(rotor), t);
    if same_bits(rebuilt.parts(), parts) {
        (rotor, Some(t.parts()), None)
    } else {
        (rotor, None, Some([parts[4], parts[5], parts[6], parts[7]]))
    }
}

/// The motor a stored form describes, or what's wrong with it.
fn motor_of_form<M>((rotor, translation, dual): MotorForm<M::Scalar>) -> Result<M, String>
where
    M: StoredMotor,
    M::Rotor: Stored<4, S = M::Scalar>,
    M::Vector: Stored<3, S = M::Scalar>,
{
    match (translation, dual) {
        (Some(_), Some(_)) => Err(format!("write `{TRANSLATION}` or `{DUAL}`, not both")),
        (_, Some(d)) => Ok(M::from_parts([
            rotor[0], rotor[1], rotor[2], rotor[3], d[0], d[1], d[2], d[3],
        ])),
        (t, None) => Ok(M::from_rotation_translation(
            <M::Rotor as Stored<4>>::from_parts(rotor),
            <M::Vector as Stored<3>>::from_parts(t.unwrap_or([M::Scalar::ZERO; 3])),
        )),
    }
}

/// Reads `{ rotor, translation }` or `{ rotor, dual }`.
pub(crate) fn motor_read<M>(v: &ValueRef<'_>, cx: &mut Reader) -> Option<M>
where
    M: StoredMotor,
    M::Rotor: Stored<4, S = M::Scalar>,
    M::Vector: Stored<3, S = M::Scalar>,
{
    let map = cx.map(v)?;
    cx.own_keys(|cx| cx.unknown_keys(map, &[ROTOR, TRANSLATION, DUAL]));
    let rotor: Option<[M::Scalar; 4]> = cx.field(map, ROTOR);
    let translation: Option<Option<[M::Scalar; 3]>> = cx.field(map, TRANSLATION);
    let dual: Option<Option<[M::Scalar; 4]>> = cx.field(map, DUAL);
    motor_of_form((rotor?, translation?, dual?))
        .map_err(|e| cx.problem(e))
        .ok()
}

/// Writes [`motor_form`] as an object.
pub(crate) fn motor_write<M>(m: &M) -> Value
where
    M: StoredMotor,
    M::Rotor: Stored<4, S = M::Scalar>,
    M::Vector: Stored<3, S = M::Scalar>,
{
    let (rotor, translation, dual) = motor_form(m);
    let mut out = Map::new();
    out.insert(ROTOR, rotor.write());
    if let Some(t) = translation {
        out.insert(TRANSLATION, t.write());
    }
    if let Some(d) = dual {
        out.insert(DUAL, d.write());
    }
    Value::Map(out)
}

/// Packs [`motor_form`] as a record, fields in the shape's order.
pub(crate) fn motor_pack<M>(m: &M, p: &mut Packer)
where
    M: StoredMotor,
    M::Rotor: Stored<4, S = M::Scalar>,
    M::Vector: Stored<3, S = M::Scalar>,
{
    let (rotor, translation, dual) = motor_form(m);
    let at = p.begin();
    rotor.pack(p);
    translation.pack(p);
    dual.pack(p);
    p.end(at);
}

/// Reads what [`motor_pack`] packed (missing trailing fields: absent).
pub(crate) fn motor_unpack<M>(u: &mut Unpacker<'_>) -> Option<M>
where
    M: StoredMotor,
    M::Rotor: Stored<4, S = M::Scalar>,
    M::Vector: Stored<3, S = M::Scalar>,
{
    let end = u.begin()?;
    let rotor = <[M::Scalar; 4]>::unpack(u)?;
    let translation = if u.more(end) {
        Option::unpack(u)?
    } else {
        None
    };
    let dual = if u.more(end) {
        Option::unpack(u)?
    } else {
        None
    };
    u.leave(end)?;
    motor_of_form((rotor, translation, dual)).ok()
}

/// `impl Schema` for a motor type, through [`motor_shape`]/[`motor_read`]/
/// [`motor_write`]/[`motor_pack`].
macro_rules! motor_schema {
    ($t:ty) => {
        impl substrate_data::Schema for $t {
            fn shape() -> substrate_data::Shape {
                $crate::data::motor_shape::<$t>()
            }
            fn read(
                v: &substrate_data::ValueRef<'_>,
                cx: &mut substrate_data::Reader,
            ) -> Option<Self> {
                $crate::data::motor_read(v, cx)
            }
            fn write(&self) -> substrate_data::Value {
                $crate::data::motor_write(self)
            }
            fn pack(&self, p: &mut substrate_data::frame::Packer) {
                $crate::data::motor_pack(self, p);
            }
            fn unpack(u: &mut substrate_data::frame::Unpacker<'_>) -> Option<Self> {
                $crate::data::motor_unpack(u)
            }
        }
    };
}
pub(crate) use motor_schema;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed_ga::{FixedBivector3, FixedMotor3, FixedRotor, FixedVec3};
    use crate::{Bivector3, Motor3, Rotor, Vec3};
    use meridian_numeric_core::Fixed;
    use substrate_data::{frame, read_bytes, to_bytes};

    /// `v` through text, binary and a frame; each back as its stored parts' bits.
    fn every_form<T: Schema + Stored<N>, const N: usize>(v: &T) -> [T; 3] {
        let text = substrate_data::to_text(v);
        let t = substrate_data::from_text::<T>(&text)
            .unwrap_or_else(|e| panic!("{e}\n{text}"))
            .value;
        let b = read_bytes::<T>(&to_bytes(v)).unwrap();
        let bytes = frame::to_frame(v);
        let f = frame::read_frame::<T>(&bytes).unwrap();
        // A named type packs as its `of`: a tool reads the frame by shape alone.
        assert!(frame::read_shaped(&bytes, &T::shape()).is_some(), "{text}");
        [t, b, f]
    }

    fn assert_bits<T: Stored<N>, const N: usize>(got: &[T], want: &T) {
        for g in got {
            assert!(
                same_bits(g.parts(), want.parts()),
                "{:?} vs {:?}",
                g.parts(),
                want.parts()
            );
        }
    }

    const TRICKY: [f32; 6] = [0.1, 1e-7, -0.0, -3.75, f32::MAX, f32::MIN_POSITIVE];

    #[test]
    fn vectors_and_bivectors_round_trip_bit_exactly() {
        for w in TRICKY.windows(3) {
            let v = Vec3::new(w[0], w[1], w[2]);
            assert_bits(&every_form(&v), &v);
            let b = Bivector3::new(w[2], w[0], w[1]);
            assert_bits(&every_form(&b), &b);
        }
        let neg_zero: Vec3 = substrate_data::from_text("[-0.0, 0, 1.5]").unwrap().value;
        assert_eq!(neg_zero.x.to_bits(), (-0.0f32).to_bits());
        assert_eq!(neg_zero.z, 1.5);
    }

    #[test]
    fn component_order_is_gac_cores() {
        // Bivector3::new takes (e23, e31, e12).
        let b = Bivector3::new(1.0, 2.0, 3.0);
        assert_eq!(
            substrate_data::to_text(&b)
                .split_whitespace()
                .collect::<String>(),
            "[1.0,2.0,3.0]"
        );
        // Oracle: R = cos(θ/2) - sin(θ/2)(n1 e23 + n2 e31 + n3 e12).
        let theta = 1.2f32;
        let (s, c) = (theta * 0.5).sin_cos();
        let about_y = Rotor::from_axis_angle(Vec3::Y, theta).parts();
        assert_eq!(about_y, [c, 0.0, -s, 0.0]);
        let about_z = Rotor::from_axis_angle(Vec3::Z, theta).parts();
        assert_eq!(about_z, [c, 0.0, 0.0, -s]);
        // And the stored rotor rotates like the original.
        let r: Rotor = substrate_data::from_text(&format!("[{c}, 0, {}, 0]", -s))
            .unwrap()
            .value;
        let p = r.transform_vector(Vec3::X);
        assert!(
            (p.x - theta.cos()).abs() < 1e-6 && (p.z + theta.sin()).abs() < 1e-6,
            "{p:?}"
        );
    }

    #[test]
    fn rotors_round_trip_bit_exactly() {
        let r = Rotor::from_axis_angle(Vec3::new(0.1, -0.7, 0.3), 2.5)
            .compose(Rotor::from_axis_angle(Vec3::X, 1e-7));
        assert_bits(&every_form(&r), &r);
        let all = every_form(&r);
        assert!(all.iter().all(|x| *x == r));
    }

    #[test]
    fn motors_round_trip_bit_exactly_in_either_written_form() {
        let translated = Motor3::translation(Vec3::new(0.1, 1.5, -0.0));
        let text = substrate_data::to_text(&translated);
        assert!(
            text.contains("translation") && !text.contains("dual"),
            "{text}"
        );
        assert_bits(&every_form(&translated), &translated);

        let r = Rotor::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7);
        let general = Motor3::from_rotation_translation(r, Vec3::new(0.1, 1e-7, -12.5))
            .compose(Motor3::rotation(Vec3::Y, 0.3));
        // Translation recovered through a rotation isn't bit-exact: `dual` is written.
        assert!(substrate_data::to_text(&general).contains("dual"));
        for m in every_form(&general) {
            assert_eq!(m, general);
        }
        assert_bits(&every_form(&general), &general);
    }

    #[test]
    fn a_written_motor_reads_as_rotation_then_translation() {
        let m: Motor3 = substrate_data::from_text(
            "{ rotor: [1, 0, 0, 0], translation: [0, 2, 0] } // hand-written",
        )
        .unwrap()
        .value;
        assert_eq!(m, Motor3::translation(Vec3::new(0.0, 2.0, 0.0)));
        let p = m.transform_point(Vec3::X);
        assert_eq!(p, Vec3::new(1.0, 2.0, 0.0));
        let only_rotor: Motor3 = substrate_data::from_text("{ rotor: [1, 0, 0, 0] }")
            .unwrap()
            .value;
        assert_eq!(only_rotor, Motor3::identity());
    }

    #[test]
    fn motor_problems_are_reported_with_their_path() {
        let e = substrate_data::from_text::<Motor3>(
            "{ rotor: [1, 0, 0, 0],\n  translation: [0, 0, 0], dual: [0, 0, 0, 0] }",
        )
        .unwrap_err();
        assert!(e.to_string().contains("not both"), "{e}");
        let e = substrate_data::from_text::<Motor3>("{\n  rotr: [1, 0, 0, 0] }").unwrap_err();
        let s = e.to_string();
        assert!(
            s.contains("2:3") && s.contains("did you mean `rotor`"),
            "{s}"
        );
        assert!(s.contains("missing `rotor`"), "{s}");
        let e = substrate_data::from_text::<Motor3>("{ rotor: [1, 0, 0] }").unwrap_err();
        assert!(e.to_string().contains("rotor: expected 4 items"), "{e}");
    }

    #[test]
    fn fixed_flavors_store_raw_bits() {
        let v = FixedVec3::new(Fixed::ZERO, Fixed::from_num(1.5), Fixed::from_bits(-1));
        let text = substrate_data::to_text(&v)
            .split_whitespace()
            .collect::<String>();
        assert_eq!(text, "[0,98304,-1]");
        assert_bits(&every_form(&v), &v);
        let b = FixedBivector3::new(Fixed::from_bits(i32::MIN), Fixed::ONE, Fixed::from_bits(7));
        assert_bits(&every_form(&b), &b);
        let r = FixedRotor::from_axis_angle(
            FixedVec3::new(Fixed::ONE, Fixed::ONE, Fixed::ZERO),
            Fixed::from_num(0.9),
        );
        assert_bits(&every_form(&r), &r);
        let m = FixedMotor3::from_rotation_translation(
            r,
            FixedVec3::new(
                Fixed::from_num(3.25),
                Fixed::from_bits(1),
                Fixed::from_num(-2.0),
            ),
        );
        assert_bits(&every_form(&m), &m);
        for back in every_form(&m) {
            assert_eq!(back, m);
        }
    }

    #[test]
    fn every_type_describes_itself_by_name() {
        let named = |s: Shape| {
            let d = s.describe();
            d.get("root")
                .and_then(|r| r.get("name"))
                .and_then(|n| n.as_str())
                .map(str::to_owned)
        };
        let names = [
            named(Vec3::shape()),
            named(Bivector3::shape()),
            named(Rotor::shape()),
            named(Motor3::shape()),
            named(FixedVec3::shape()),
            named(FixedBivector3::shape()),
            named(FixedRotor::shape()),
            named(FixedMotor3::shape()),
        ];
        let want = [
            "vec3",
            "bivector3",
            "rotor",
            "motor",
            "fixed_vec3",
            "fixed_bivector3",
            "fixed_rotor",
            "fixed_motor",
        ];
        assert_eq!(names.map(|n| n.unwrap()), want);
        let motor = Motor3::shape().describe();
        let fields = motor
            .get("defs")
            .and_then(|d| d.get("motor"))
            .and_then(|m| m.get("fields"));
        assert_eq!(fields.and_then(|f| f.as_list()).map(<[_]>::len), Some(3));
    }
}
