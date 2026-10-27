//! Float-precision (`f32`) projective geometric algebra: vectors, bivectors,
//! multivectors, rotors and motors — the default, GPU-dispatchable spatial
//! math every subsystem builds on. See `crate::fixed_ga` for the
//! deterministic (`Fixed`, non-GPU) alternative used by
//! `physics-core`'s opt-in deterministic simulation path.
//!
//! Built on 3D projective geometric algebra (PGA), the algebra R(3,0,1):
//! four basis vectors `e0` (ideal/degenerate, `e0^2 = 0`) and `e1`, `e2`,
//! `e3` (Euclidean, `ei^2 = 1`), anticommuting pairwise. A [`Multivector`]
//! is indexed by a 4-bit blade mask (bit i = whether `ei` is present); see
//! the `blade` module for named indices. See docs/gac-design.md and
//! [ADR 001](../../../docs/adr/001-geometric-algebra-as-spatial-model.md).

use core::ops::{Add, Mul, Neg, Sub};

use meridian_numeric_core::Scalar;

use crate::blade;

/// Geometric product of two basis blades, given as 4-bit masks: reorders
/// the concatenated factors into canonical increasing order (bubble sort,
/// tracking the sign of the permutation) and contracts adjacent equal
/// factors via the metric (`e0*e0 = 0`, `ei*ei = 1` for i=1,2,3). Returns
/// `(0.0, _)` when the product vanishes (an `e0` factor met itself).
fn basis_product(a: u8, b: u8) -> (Scalar, u8) {
    let mut seq: [u8; 8] = [0; 8];
    let mut len = 0usize;
    for i in 0..4u8 {
        if a & (1 << i) != 0 {
            seq[len] = i;
            len += 1;
        }
    }
    for i in 0..4u8 {
        if b & (1 << i) != 0 {
            seq[len] = i;
            len += 1;
        }
    }

    let mut sign: Scalar = 1.0;
    loop {
        let mut swapped = false;
        for i in 0..len.saturating_sub(1) {
            if seq[i] > seq[i + 1] {
                seq.swap(i, i + 1);
                sign = -sign;
                swapped = true;
            }
        }

        let mut contracted = false;
        let mut i = 0;
        while i + 1 < len {
            if seq[i] == seq[i + 1] {
                if seq[i] == 0 {
                    return (0.0, 0); // e0 * e0 = 0
                }
                for k in i..len - 2 {
                    seq[k] = seq[k + 2];
                }
                len -= 2;
                contracted = true;
            } else {
                i += 1;
            }
        }

        if !swapped && !contracted {
            break;
        }
    }

    let mut mask = 0u8;
    for &b in seq.iter().take(len) {
        mask |= 1 << b;
    }
    (sign, mask)
}

/// The general element of the algebra: closed under the geometric product.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Multivector(pub [Scalar; 16]);

impl Multivector {
    pub const ZERO: Multivector = Multivector([0.0; 16]);

    pub fn scalar(s: Scalar) -> Self {
        let mut m = [0.0; 16];
        m[blade::S] = s;
        Multivector(m)
    }

    /// Grade reversal: negates each blade by `(-1)^(k*(k-1)/2)` for its
    /// grade `k` (the number of basis vectors it's built from). Used to
    /// build the "conjugate" side of a sandwich product.
    pub fn reverse(self) -> Self {
        let mut out = self.0;
        for (i, v) in out.iter_mut().enumerate() {
            let k = (i as u32).count_ones();
            // grades 0,1 -> +1; grades 2,3 -> -1; grade 4 -> +1
            if matches!(k, 2 | 3) {
                *v = -*v;
            }
        }
        Multivector(out)
    }
}

impl Add for Multivector {
    type Output = Multivector;
    fn add(self, rhs: Multivector) -> Multivector {
        let mut out = self.0;
        for (o, r) in out.iter_mut().zip(rhs.0) {
            *o += r;
        }
        Multivector(out)
    }
}

impl Sub for Multivector {
    type Output = Multivector;
    fn sub(self, rhs: Multivector) -> Multivector {
        let mut out = self.0;
        for (o, r) in out.iter_mut().zip(rhs.0) {
            *o -= r;
        }
        Multivector(out)
    }
}

impl Neg for Multivector {
    type Output = Multivector;
    fn neg(self) -> Multivector {
        let mut out = self.0;
        for v in &mut out {
            *v = -*v;
        }
        Multivector(out)
    }
}

impl Mul<Scalar> for Multivector {
    type Output = Multivector;
    fn mul(self, rhs: Scalar) -> Multivector {
        let mut out = self.0;
        for v in &mut out {
            *v *= rhs;
        }
        Multivector(out)
    }
}

impl Mul for Multivector {
    type Output = Multivector;
    /// The geometric product — the one operation the whole algebra (and
    /// every type in this crate) is built from.
    fn mul(self, rhs: Multivector) -> Multivector {
        let mut out = [0.0 as Scalar; 16];
        for a in 0..16u8 {
            let lhs_val = self.0[a as usize];
            if lhs_val == 0.0 {
                continue;
            }
            for b in 0..16u8 {
                let rhs_val = rhs.0[b as usize];
                if rhs_val == 0.0 {
                    continue;
                }
                let (sign, mask) = basis_product(a, b);
                if sign == 0.0 {
                    continue;
                }
                out[mask as usize] += sign * lhs_val * rhs_val;
            }
        }
        Multivector(out)
    }
}

/// A plain 3D Euclidean vector — the ergonomic type most call sites use
/// (points, directions, axes, translations); [`Multivector`]/[`Motor3`]
/// are the machinery underneath, not the day-to-day API.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: Scalar,
    pub y: Scalar,
    pub z: Scalar,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const X: Vec3 = Vec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: Vec3 = Vec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    pub const fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, rhs: Vec3) -> Scalar {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Vec3::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    pub fn length_squared(self) -> Scalar {
        self.dot(self)
    }

    pub fn length(self) -> Scalar {
        self.length_squared().sqrt()
    }

    /// Returns `self` unchanged if it's shorter than
    /// [`meridian_numeric_core::EPSILON`] rather than dividing by ~zero.
    pub fn normalize(self) -> Vec3 {
        let len = self.length();
        if len <= meridian_numeric_core::EPSILON {
            self
        } else {
            self * (1.0 / len)
        }
    }

    #[cfg(test)]
    pub(crate) fn approx_eq(self, rhs: Vec3) -> bool {
        meridian_numeric_core::approx_eq(self.x, rhs.x)
            && meridian_numeric_core::approx_eq(self.y, rhs.y)
            && meridian_numeric_core::approx_eq(self.z, rhs.z)
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<Scalar> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: Scalar) -> Vec3 {
        Vec3::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

/// A single k-vector term (scalar, vector, bivector, ...).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Blade {
    pub grade: u8,
    pub value: Scalar,
}

/// A bivector in the Euclidean part of the algebra (`e23`, `e31`, `e12`
/// components) — the GA-native representation of angular velocity and
/// rotation generators. Deliberately a distinct type from [`Vec3`]:
/// angular velocity lives in the Lie algebra so(3), which *is* the space
/// of these bivectors, not the space of vectors. Cross-product-based
/// "angular velocity as a vector" conflates the two because in 3D a
/// bivector's Hodge dual happens to have the same 3 components as a
/// vector — a coincidence special to 3D that GA makes explicit instead of
/// hiding (see [`Bivector3::wedge`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bivector3 {
    pub e23: Scalar,
    pub e31: Scalar,
    pub e12: Scalar,
}

impl Bivector3 {
    pub const ZERO: Bivector3 = Bivector3 {
        e23: 0.0,
        e31: 0.0,
        e12: 0.0,
    };

    pub const fn new(e23: Scalar, e31: Scalar, e12: Scalar) -> Self {
        Self { e23, e31, e12 }
    }

    /// The wedge product `a ∧ b`, e.g. torque = `Bivector3::wedge(r, f)`
    /// for a force `f` applied at offset `r` from a pivot. Numerically
    /// identical to `a.cross(b)` — the "coincidence" `Bivector3`'s own
    /// doc comment mentions — but returns a `Bivector3`, not a `Vec3`,
    /// because a torque *is* a bivector quantity, not a vector one.
    pub fn wedge(a: Vec3, b: Vec3) -> Bivector3 {
        Bivector3::new(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x,
        )
    }

    pub fn length(self) -> Scalar {
        (self.e23 * self.e23 + self.e31 * self.e31 + self.e12 * self.e12).sqrt()
    }

    /// `exp(B)` for a rotation-generator bivector: reduces to
    /// [`Rotor::from_axis_angle`] (a rotation-only bivector's exponential
    /// *is* a rotor, by definition — no separate derivation needed, this
    /// delegates to the already-verified formula) with axis = the
    /// bivector's direction and angle = its magnitude. Used to integrate
    /// a rigid body's orientation over a timestep: `(angular_velocity *
    /// dt).exp()` gives the rotor to compose onto the body's frame — the
    /// GA equivalent of quaternion exponential-map integration, and for
    /// the same reason: it stays exactly on the unit-rotor manifold, no
    /// drift/renormalization the way separately integrating three Euler
    /// angles would.
    pub fn exp(self) -> Rotor {
        let angle = self.length();
        if angle <= meridian_numeric_core::EPSILON {
            return Rotor::identity();
        }
        let axis = Vec3::new(self.e23, self.e31, self.e12) * (1.0 / angle);
        Rotor::from_axis_angle(axis, angle)
    }
}

impl Add for Bivector3 {
    type Output = Bivector3;
    fn add(self, rhs: Bivector3) -> Bivector3 {
        Bivector3::new(self.e23 + rhs.e23, self.e31 + rhs.e31, self.e12 + rhs.e12)
    }
}

impl Sub for Bivector3 {
    type Output = Bivector3;
    fn sub(self, rhs: Bivector3) -> Bivector3 {
        Bivector3::new(self.e23 - rhs.e23, self.e31 - rhs.e31, self.e12 - rhs.e12)
    }
}

impl Mul<Scalar> for Bivector3 {
    type Output = Bivector3;
    fn mul(self, rhs: Scalar) -> Bivector3 {
        Bivector3::new(self.e23 * rhs, self.e31 * rhs, self.e12 * rhs)
    }
}

/// A pure rotation, the even-graded (scalar + bivector) subalgebra element
/// `cos(θ/2) - sin(θ/2) * (n1*e23 + n2*e31 + n3*e12)` for a unit axis `n`
/// and angle `θ` — geometric algebra's equivalent of a unit quaternion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotor(pub Multivector);

impl Default for Rotor {
    fn default() -> Self {
        Rotor::identity()
    }
}

impl Rotor {
    pub fn identity() -> Self {
        Rotor(Multivector::scalar(1.0))
    }

    /// A rotation by `angle` radians about `axis` (need not be
    /// pre-normalized). Right-handed: rotating [`Vec3::X`] about
    /// [`Vec3::Z`] by a positive angle sweeps it toward [`Vec3::Y`].
    pub fn from_axis_angle(axis: Vec3, angle: Scalar) -> Self {
        let axis = axis.normalize();
        let half = angle * 0.5;
        let (s, c) = (half.sin(), half.cos());
        let mut m = [0.0; 16];
        m[blade::S] = c;
        m[blade::E23] = -s * axis.x;
        m[blade::E13] = s * axis.y; // e31 = -e13, stored in canonical e13
        m[blade::E12] = -s * axis.z;
        Rotor(Multivector(m))
    }

    /// Composes two rotations: `self` applied first, then `rhs` — i.e.
    /// `rhs.compose(self)` in row-vector convention, `self` in column-
    /// vector/geometric-product convention. Matches [`Motor3::compose`].
    pub fn compose(self, rhs: Rotor) -> Rotor {
        Rotor(rhs.0 * self.0)
    }

    pub fn reverse(self) -> Rotor {
        Rotor(self.0.reverse())
    }

    /// Rotates a plain Euclidean vector via the sandwich product `R v R~`.
    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        let mut vm = [0.0; 16];
        vm[blade::E1] = v.x;
        vm[blade::E2] = v.y;
        vm[blade::E3] = v.z;
        let p = Multivector(vm);
        let r = self.0 * p * self.0.reverse();
        Vec3::new(r.0[blade::E1], r.0[blade::E2], r.0[blade::E3])
    }
}

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

/// Column-major 4x4 matrix multiply (`lhs * rhs`, column-vector
/// convention) on the raw arrays [`Motor3::to_mat4`]/[`crate::Projection`]
/// already return. Lives here, not in `graphics-core`: it started as a
/// `graphics-core`-private helper (composing `Camera`'s view-projection
/// matrix), with a note on itself that a second consumer needing generic
/// mat4 multiply was the signal to move it down — a windowed rendering
/// example composing a per-object model matrix with `Camera`'s
/// view-projection is exactly that second consumer. `numeric-core` (the
/// other candidate) turned out not to fit: it's scalar-only (`Scalar`/
/// `Fixed`, no matrix machinery at all), while `to_mat4` and `Projection`
/// already established `gac-core::float_ga` as where this workspace's
/// classical-matrix graphics bridging math lives.
pub fn mat4_mul(lhs: [[Scalar; 4]; 4], rhs: [[Scalar; 4]; 4]) -> [[Scalar; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for (col, rhs_col) in rhs.iter().enumerate() {
        for row in 0..4 {
            out[col][row] = (0..4).map(|k| lhs[k][row] * rhs_col[k]).sum();
        }
    }
    out
}

// ---- GaFlavor: lets generic code (this crate's primitives, physics-core's
// engine) be written once against `f32` and `Fixed` both ----

use crate::generic::{BivectorLike, GaFlavor, MotorLike, RotorLike, ScalarLike, VectorLike};

/// The default GA flavor: `f32`-backed, GPU-dispatchable. See the crate
/// root doc comment for what "flavor" means here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FloatFlavor;

impl ScalarLike for Scalar {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const EPSILON: Self = meridian_numeric_core::EPSILON;
    const MAX: Self = f32::MAX;

    fn from_f64(v: f64) -> Self {
        v as f32
    }
    fn sqrt(self) -> Self {
        self.sqrt()
    }
    fn tan(self) -> Self {
        self.tan()
    }
    fn sin_cos(self) -> (Self, Self) {
        self.sin_cos()
    }
    fn signum(self) -> Self {
        self.signum()
    }
    fn abs(self) -> Self {
        self.abs()
    }
    fn min(self, other: Self) -> Self {
        self.min(other)
    }
    fn max(self, other: Self) -> Self {
        self.max(other)
    }
    fn clamp(self, lo: Self, hi: Self) -> Self {
        self.clamp(lo, hi)
    }
}

impl VectorLike for Vec3 {
    type Scalar = Scalar;

    const ZERO: Self = Vec3::ZERO;

    fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Vec3::new(x, y, z)
    }
    fn x(self) -> Scalar {
        self.x
    }
    fn y(self) -> Scalar {
        self.y
    }
    fn z(self) -> Scalar {
        self.z
    }
    fn dot(self, rhs: Self) -> Scalar {
        self.dot(rhs)
    }
    fn cross(self, rhs: Self) -> Self {
        self.cross(rhs)
    }
    fn length(self) -> Scalar {
        self.length()
    }
    fn length_squared(self) -> Scalar {
        self.length_squared()
    }
    fn normalize(self) -> Self {
        self.normalize()
    }
}

impl BivectorLike for Bivector3 {
    type Scalar = Scalar;
    type Vector = Vec3;
    type Rotor = Rotor;

    const ZERO: Self = Bivector3::ZERO;

    fn wedge(a: Vec3, b: Vec3) -> Self {
        Bivector3::wedge(a, b)
    }
    fn length(self) -> Scalar {
        self.length()
    }
    fn exp(self) -> Rotor {
        self.exp()
    }
}

impl RotorLike for Rotor {
    type Scalar = Scalar;
    type Vector = Vec3;

    fn identity() -> Self {
        Rotor::identity()
    }
    fn from_axis_angle(axis: Vec3, angle: Scalar) -> Self {
        Rotor::from_axis_angle(axis, angle)
    }
    fn compose(self, rhs: Self) -> Self {
        self.compose(rhs)
    }
    fn reverse(self) -> Self {
        self.reverse()
    }
    fn transform_vector(self, v: Vec3) -> Vec3 {
        self.transform_vector(v)
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

impl GaFlavor for FloatFlavor {
    type Scalar = Scalar;
    type Vector = Vec3;
    type Bivector = Bivector3;
    type Rotor = Rotor;
    type Motor = Motor3;
}

/// A single point, treated as a degenerate shape: its own support point
/// regardless of direction. See `crate::Shape`'s doc comment; this can't
/// be a blanket `impl<F: GaFlavor> Shape<F> for F::Vector` in `lib.rs` for
/// coherence reasons (see the comment there), so it's concrete per flavor
/// instead.
impl crate::generic::Shape<FloatFlavor> for Vec3 {
    fn support(&self, _direction: Vec3) -> Vec3 {
        *self
    }
}

// ---- Thin aliases for the generic primitives in `crate::lib` ----
//
// `Frame`/`Shape`/`Aabb`/`Sphere`/`Obb`/`Cone`/`Plane`/`ConvexVolume`/
// `Projection` are written once, generically over `GaFlavor` (see the
// crate root doc comment for why) — these aliases are what let existing
// call sites keep writing `meridian_gac_core::Aabb` etc. unchanged.

// `Shape` itself can't get a same-style alias (`Shape<F>` is a trait, and
// Rust's stable trait aliases don't exist yet) — downstream code bounds
// generic parameters as `S: meridian_gac_core::Shape<FloatFlavor>`
// directly (`meridian_gac_core::Shape` and `FloatFlavor` are both already
// re-exported at the crate root).
pub type Frame = crate::generic::Frame<FloatFlavor>;
pub type Aabb = crate::generic::Aabb<FloatFlavor>;
pub type Sphere = crate::generic::Sphere<FloatFlavor>;
pub type Obb = crate::generic::Obb<FloatFlavor>;
pub type Cone = crate::generic::Cone<FloatFlavor>;
pub type Plane = crate::generic::Plane<FloatFlavor>;
pub type ConvexVolume = crate::generic::ConvexVolume<FloatFlavor>;
pub type Projection = crate::generic::Projection<FloatFlavor>;
pub type Mesh = crate::generic::Mesh<FloatFlavor>;

/// See [`crate::generic::icosphere`].
pub fn icosphere(subdivisions: u32) -> Mesh {
    crate::generic::icosphere::<FloatFlavor>(subdivisions)
}

// ---- Cross-flavor interop with `crate::fixed_ga` ----
//
// Every conversion here is precision-changing (`f32` <-> `Fixed`'s
// ~1.5e-5 fixed resolution) and, going float -> fixed, changes how the
// value behaves under further computation (gains the determinism
// guarantee; going fixed -> float, loses it). Deliberately *not*
// `From`/`Into`: those traits make a cast look free at the call site
// (`.into()` gives no hint anything precision- or determinism-relevant
// happened), which is exactly wrong for a conversion this consequential.
// Every method here is named `_lossy` instead, so every call site says
// out loud what it's doing — see docs/gac-design.md's "float_ga/fixed_ga
// interop" section. The `Add`/`Sub`/`Mul` impls below (for mixing a
// `Vec3` with a `FixedVec3` directly) go through these same named
// methods internally, not a silent implicit conversion.
use meridian_numeric_core::Fixed;

use crate::fixed_ga::{FixedBivector3, FixedMotor3, FixedMultivector, FixedRotor, FixedVec3};

impl Vec3 {
    /// Deliberate precision-changing cast to the deterministic flavor.
    pub fn to_fixed_lossy(self) -> FixedVec3 {
        FixedVec3::new(
            Fixed::from_num(self.x as f64),
            Fixed::from_num(self.y as f64),
            Fixed::from_num(self.z as f64),
        )
    }
}

impl Bivector3 {
    /// Deliberate precision-changing cast to the deterministic flavor.
    pub fn to_fixed_lossy(self) -> FixedBivector3 {
        FixedBivector3::new(
            Fixed::from_num(self.e23 as f64),
            Fixed::from_num(self.e31 as f64),
            Fixed::from_num(self.e12 as f64),
        )
    }
}

impl Rotor {
    /// Deliberate precision-changing cast to the deterministic flavor —
    /// component-wise across the underlying multivector (both `Rotor`
    /// and `FixedRotor` expose theirs, `pub`), not a re-derivation
    /// through axis/angle.
    pub fn to_fixed_lossy(self) -> FixedRotor {
        let mut out = [Fixed::ZERO; 16];
        for (dst, src) in out.iter_mut().zip(self.0.0) {
            *dst = Fixed::from_num(src as f64);
        }
        FixedRotor(FixedMultivector(out))
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

impl Add<FixedVec3> for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: FixedVec3) -> Vec3 {
        self + rhs.to_float_lossy()
    }
}

impl Sub<FixedVec3> for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: FixedVec3) -> Vec3 {
        self - rhs.to_float_lossy()
    }
}

impl Mul<Fixed> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: Fixed) -> Vec3 {
        self * (rhs.to_num() as f32)
    }
}

#[cfg(test)]
mod tests;
