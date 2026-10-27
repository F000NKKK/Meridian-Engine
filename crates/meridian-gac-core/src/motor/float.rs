//! The float (`f32`) motor: rotation plus translation, GPU-dispatchable.

use meridian_numeric_core::{Fixed, Scalar};

use super::fixed::FixedMotor3;
use super::frame;
use crate::blade;
use crate::flavor::MotorLike;
use crate::flavor::float::FloatFlavor;
use crate::multivector::fixed::FixedMultivector;
use crate::multivector::float::Multivector;
use crate::rotor::float::Rotor;
use crate::vector::float::Vec3;

/// Rotation + translation, composable via the geometric product. This is
/// what `Transform` is built from workspace-wide — see docs/gac-design.md.
/// Represented as `translator * rotor` in the even subalgebra: applying a
/// `Motor3` to a point rotates it first, then translates it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motor3(pub Multivector);

/// Row-major 3x3 matrix a motor's rotor blades expand to.
struct LinearPart([[Scalar; 3]; 3]);

impl LinearPart {
    fn apply(&self, v: Vec3) -> Vec3 {
        let [r0, r1, r2] = &self.0;
        Vec3::new(
            r0[0] * v.x + r0[1] * v.y + r0[2] * v.z,
            r1[0] * v.x + r1[1] * v.y + r1[2] * v.z,
            r2[0] * v.x + r2[1] * v.y + r2[2] * v.z,
        )
    }
}

impl Default for Motor3 {
    fn default() -> Self {
        Motor3::identity()
    }
}

impl Motor3 {
    pub fn identity() -> Self {
        Motor3(Multivector::scalar(1.0))
    }

    pub fn rotation(axis: Vec3, angle: Scalar) -> Self {
        Motor3(Rotor::from_axis_angle(axis, angle).0)
    }

    pub fn translation(t: Vec3) -> Self {
        let mut m = [0.0; 16];
        m[blade::S] = 1.0;
        m[blade::E01] = -t.x * 0.5;
        m[blade::E02] = -t.y * 0.5;
        m[blade::E03] = -t.z * 0.5;
        Motor3(Multivector(m))
    }

    /// Builds a motor that rotates first, then translates — the same
    /// order [`Motor3::transform_point`] applies.
    pub fn from_rotation_translation(rotor: Rotor, t: Vec3) -> Self {
        Motor3::translation(t).compose_raw(Motor3(rotor.0))
    }

    /// Composes two motors: `self` applied first, then `rhs`
    /// (`result.transform_point(p) == rhs.transform_point(self.transform_point(p))`).
    pub fn compose(self, rhs: Motor3) -> Motor3 {
        Motor3(rhs.0 * self.0)
    }

    fn compose_raw(self, rhs: Motor3) -> Motor3 {
        Motor3(self.0 * rhs.0)
    }

    /// The inverse motion: for a unit motor (every motor built by this
    /// crate's constructors is one), the inverse is the grade reversal.
    pub fn inverse(self) -> Motor3 {
        Motor3(self.0.reverse())
    }

    /// Transforms a point through this rigid motion: the sandwich product
    /// `M P M~`, where `P` is `point` embedded as the PGA trivector
    /// `e123 - x*e023 + y*e013 - z*e012`.
    ///
    /// The product is expanded once by hand instead of multiplying two
    /// 16-component multivectors: a motor only has the eight even blades and
    /// a point only four, so the result is the affine map
    /// `linear_part() * point + origin()`, a few dozen multiplications.
    pub fn transform_point(self, point: Vec3) -> Vec3 {
        self.linear_part().apply(point) + self.origin()
    }

    /// Transforms a direction (not a point) through this rigid motion's
    /// rotational part only — translation has no effect on a direction.
    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        self.linear_part().apply(v)
    }

    /// Where this motor sends the origin: the translation of the affine map
    /// `transform_point` evaluates.
    fn origin(self) -> Vec3 {
        let m = &self.0.0;
        let (s, e12, e13, e23) = (m[blade::S], m[blade::E12], m[blade::E13], m[blade::E23]);
        let (e01, e02, e03, e0123) = (m[blade::E01], m[blade::E02], m[blade::E03], m[blade::E0123]);
        Vec3::new(
            -2.0 * (s * e01 + e12 * e02 + e13 * e03 + e23 * e0123),
            -2.0 * (s * e02 - e12 * e01 - e13 * e0123 + e23 * e03),
            -2.0 * (s * e03 + e12 * e0123 - e13 * e01 - e23 * e02),
        )
    }

    /// The rotational part of the affine map `transform_point` evaluates,
    /// from the four rotor blades (`s`, `e12`, `e13`, `e23`) only.
    fn linear_part(self) -> LinearPart {
        let m = &self.0.0;
        let (s, e12, e13, e23) = (m[blade::S], m[blade::E12], m[blade::E13], m[blade::E23]);
        let (ss, aa, bb, cc) = (s * s, e12 * e12, e13 * e13, e23 * e23);
        let (s12, s13, s23) = (2.0 * s * e12, 2.0 * s * e13, 2.0 * s * e23);
        let (p13_23, p12_23, p12_13) = (2.0 * e13 * e23, 2.0 * e12 * e23, 2.0 * e12 * e13);
        LinearPart([
            [ss - aa - bb + cc, s12 - p13_23, s13 + p12_23],
            [-s12 - p13_23, ss - aa + bb - cc, s23 - p12_13],
            [p12_23 - s13, -s23 - p12_13, ss + aa - bb - cc],
        ])
    }

    /// The equivalent column-major homogeneous 4x4 matrix: `to_mat4()[c][r]`
    /// is column `c`, row `r`, so that `M * [x, y, z, 1]^T` (column-vector
    /// convention, matching wgpu/GLSL) reproduces `self.transform_point`.
    ///
    /// Built directly from `transform_point` rather than by pulling rotor
    /// coefficients out of the multivector by hand: `transform_point` is an
    /// affine map (rotation then translation), so evaluating it at the
    /// origin and at each basis vector is an exact reconstruction of that
    /// map's matrix, not a numerical approximation. This is the one place
    /// `gac-core` produces a classical matrix — graphics APIs need one, but
    /// the conversion stays generic bridging math, not a graphics concept
    /// (see docs/gac-design.md), so it belongs here rather than being
    /// re-derived independently in `graphics-core`.
    pub fn to_mat4(self) -> [[Scalar; 4]; 4] {
        let [r0, r1, r2] = self.linear_part().0;
        let origin = self.origin();
        [
            [r0[0], r1[0], r2[0], 0.0],
            [r0[1], r1[1], r2[1], 0.0],
            [r0[2], r1[2], r2[2], 0.0],
            [origin.x, origin.y, origin.z, 1.0],
        ]
    }
}

impl MotorLike for Motor3 {
    type Scalar = Scalar;
    type Vector = Vec3;
    type Rotor = Rotor;

    fn identity() -> Self {
        Motor3::identity()
    }
    fn translation(t: Vec3) -> Self {
        Motor3::translation(t)
    }
    fn from_rotation_translation(rotor: Rotor, t: Vec3) -> Self {
        Motor3::from_rotation_translation(rotor, t)
    }
    fn compose(self, rhs: Self) -> Self {
        self.compose(rhs)
    }
    fn inverse(self) -> Self {
        self.inverse()
    }
    fn transform_point(self, p: Vec3) -> Vec3 {
        self.transform_point(p)
    }
    fn transform_vector(self, v: Vec3) -> Vec3 {
        self.transform_vector(v)
    }
}

impl Motor3 {
    /// Deliberate precision-changing cast to the deterministic flavor —
    /// component-wise across the underlying multivector, not a
    /// re-derivation through rotation/translation.
    pub fn to_fixed_lossy(self) -> FixedMotor3 {
        let mut out = [Fixed::ZERO; 16];
        for (dst, src) in out.iter_mut().zip(self.0.0) {
            *dst = Fixed::from_num(src as f64);
        }
        FixedMotor3(FixedMultivector(out))
    }
}

/// A named reference frame in the float flavor.
pub type Frame = frame::Frame<FloatFlavor>;
