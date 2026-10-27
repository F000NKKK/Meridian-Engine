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

    /// Transforms a point through this rigid motion via the sandwich
    /// product `M P M~`, where `P` is `point` embedded as the PGA
    /// trivector `e123 - x*e023 + y*e013 - z*e012`.
    pub fn transform_point(self, point: Vec3) -> Vec3 {
        let mut pm = [0.0; 16];
        pm[blade::E123] = 1.0;
        pm[blade::E023] = -point.x;
        pm[blade::E013] = point.y;
        pm[blade::E012] = -point.z;
        let p = Multivector(pm);
        let r = self.0 * p * self.0.reverse();
        Vec3::new(-r.0[blade::E023], r.0[blade::E013], -r.0[blade::E012])
    }

    /// Transforms a direction (not a point) through this rigid motion's
    /// rotational part only — translation has no effect on a direction.
    /// Computed exactly as `transform_point(v) - transform_point(ZERO)`:
    /// `transform_point` is an affine map, so the translation term cancels
    /// out of that difference algebraically, leaving exactly the linear
    /// (rotational) part — not an approximation, the same reasoning
    /// [`to_mat4`](Self::to_mat4) uses to isolate each basis column.
    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        self.transform_point(v) - self.transform_point(Vec3::ZERO)
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
        let origin = self.transform_point(Vec3::ZERO);
        let x_axis = self.transform_point(Vec3::X) - origin;
        let y_axis = self.transform_point(Vec3::Y) - origin;
        let z_axis = self.transform_point(Vec3::Z) - origin;
        [
            [x_axis.x, x_axis.y, x_axis.z, 0.0],
            [y_axis.x, y_axis.y, y_axis.z, 0.0],
            [z_axis.x, z_axis.y, z_axis.z, 0.0],
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
