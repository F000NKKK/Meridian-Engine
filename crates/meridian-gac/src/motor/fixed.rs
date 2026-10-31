//! The fixed-point (`Fixed`) motor, mirroring [`super::float`].

use meridian_numeric::Fixed;

use super::float::Motor3;
use super::frame;
use crate::blade;
use crate::flavor::MotorLike;
use crate::flavor::fixed::FixedFlavor;
use crate::multivector::fixed::FixedMultivector;
use crate::multivector::float::Multivector;
use crate::rotor::fixed::FixedRotor;
use crate::vector::fixed::FixedVec3;

/// Mirrors [`crate::Motor3`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedMotor3(pub FixedMultivector);

impl Default for FixedMotor3 {
    fn default() -> Self {
        FixedMotor3::identity()
    }
}

impl FixedMotor3 {
    /// Returns the identity rigid motion.
    pub fn identity() -> Self {
        FixedMotor3(FixedMultivector::scalar(Fixed::ONE))
    }

    /// Constructs a pure translation.
    pub fn translation(t: FixedVec3) -> Self {
        let mut m = [Fixed::ZERO; 16];
        m[blade::S] = Fixed::ONE;
        let half = Fixed::from_num(0.5);
        m[blade::E01] = -t.x * half;
        m[blade::E02] = -t.y * half;
        m[blade::E03] = -t.z * half;
        FixedMotor3(FixedMultivector(m))
    }

    /// Constructs a rotation followed by a translation.
    pub fn from_rotation_translation(rotor: FixedRotor, t: FixedVec3) -> Self {
        FixedMotor3::translation(t).compose_raw(FixedMotor3(rotor.0))
    }

    /// Applies this motion first and `rhs` second.
    pub fn compose(self, rhs: FixedMotor3) -> FixedMotor3 {
        FixedMotor3(rhs.0 * self.0)
    }

    fn compose_raw(self, rhs: FixedMotor3) -> FixedMotor3 {
        FixedMotor3(self.0 * rhs.0)
    }

    /// Returns the inverse rigid motion.
    pub fn inverse(self) -> FixedMotor3 {
        FixedMotor3(self.0.reverse())
    }

    /// Transforms a point. Mirrors [`crate::Motor3::transform_point`]: the
    /// affine map `linear_part() * point + origin()` rather than the full
    /// product `M P M~` over 16-component multivectors. The result can differ
    /// from that product by a few units of the last place (different
    /// truncation points), and is bit-identical on every platform.
    pub fn transform_point(self, point: FixedVec3) -> FixedVec3 {
        self.linear_part().apply(point) + self.origin()
    }

    /// Mirrors [`crate::Motor3::transform_vector`]: the rotation-only action
    /// on a direction, so translation has no effect.
    pub fn transform_vector(self, v: FixedVec3) -> FixedVec3 {
        self.linear_part().apply(v)
    }

    /// Where this motor sends the origin.
    fn origin(self) -> FixedVec3 {
        let m = &self.0.0;
        let (s, e12, e13, e23) = (m[blade::S], m[blade::E12], m[blade::E13], m[blade::E23]);
        let (e01, e02, e03, e0123) = (m[blade::E01], m[blade::E02], m[blade::E03], m[blade::E0123]);
        FixedVec3::new(
            -twice(s * e01 + e12 * e02 + e13 * e03 + e23 * e0123),
            -twice(s * e02 - e12 * e01 - e13 * e0123 + e23 * e03),
            -twice(s * e03 + e12 * e0123 - e13 * e01 - e23 * e02),
        )
    }

    /// The rotational part of the affine map, from the four rotor blades.
    fn linear_part(self) -> LinearPart {
        let m = &self.0.0;
        let (s, e12, e13, e23) = (m[blade::S], m[blade::E12], m[blade::E13], m[blade::E23]);
        let (ss, aa, bb, cc) = (s * s, e12 * e12, e13 * e13, e23 * e23);
        let (s12, s13, s23) = (twice(s * e12), twice(s * e13), twice(s * e23));
        let (p13_23, p12_23, p12_13) = (twice(e13 * e23), twice(e12 * e23), twice(e12 * e13));
        LinearPart([
            [ss - aa - bb + cc, s12 - p13_23, s13 + p12_23],
            [-s12 - p13_23, ss - aa + bb - cc, s23 - p12_13],
            [p12_23 - s13, -s23 - p12_13, ss + aa - bb - cc],
        ])
    }
}

/// `2 * x` without a multiplication.
fn twice(x: Fixed) -> Fixed {
    x + x
}

/// A 3x3 rotation matrix, row-major.
struct LinearPart([[Fixed; 3]; 3]);

impl LinearPart {
    fn apply(&self, v: FixedVec3) -> FixedVec3 {
        let [r0, r1, r2] = &self.0;
        FixedVec3::new(
            r0[0] * v.x + r0[1] * v.y + r0[2] * v.z,
            r1[0] * v.x + r1[1] * v.y + r1[2] * v.z,
            r2[0] * v.x + r2[1] * v.y + r2[2] * v.z,
        )
    }
}

impl MotorLike for FixedMotor3 {
    type Scalar = Fixed;
    type Vector = FixedVec3;
    type Rotor = FixedRotor;

    fn identity() -> Self {
        FixedMotor3::identity()
    }
    fn translation(t: FixedVec3) -> Self {
        FixedMotor3::translation(t)
    }
    fn from_rotation_translation(rotor: FixedRotor, t: FixedVec3) -> Self {
        FixedMotor3::from_rotation_translation(rotor, t)
    }
    fn compose(self, rhs: Self) -> Self {
        self.compose(rhs)
    }
    fn inverse(self) -> Self {
        self.inverse()
    }
    fn transform_point(self, p: FixedVec3) -> FixedVec3 {
        self.transform_point(p)
    }
    fn transform_vector(self, v: FixedVec3) -> FixedVec3 {
        self.transform_vector(v)
    }
}

impl FixedMotor3 {
    /// Deliberate precision-changing cast to the default flavor —
    /// component-wise across the underlying multivector, not a
    /// re-derivation through rotation/translation. The way a
    /// `physics-domain::fixed::RigidBody`'s pose reaches rendering/ECS/audio,
    /// which stay `f32` regardless of which physics path produced the pose.
    pub fn to_float_lossy(self) -> Motor3 {
        let mut out = [0.0f32; 16];
        for (dst, src) in out.iter_mut().zip(self.0.0) {
            *dst = src.to_num() as f32;
        }
        Motor3(Multivector(out))
    }
}

/// A named reference frame in the fixed-point flavor.
pub type FixedFrame = frame::Frame<FixedFlavor>;
