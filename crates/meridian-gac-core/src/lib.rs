//! Geometric Algebra Core — vectors, bivectors, multivectors, rotors and motors; the shared spatial math foundation for every other subsystem.
//!
//! Built on 3D projective geometric algebra (PGA), the algebra R(3,0,1):
//! four basis vectors `e0` (ideal/degenerate, `e0^2 = 0`) and `e1`, `e2`,
//! `e3` (Euclidean, `ei^2 = 1`), anticommuting pairwise. A [`Multivector`]
//! is indexed by a 4-bit blade mask (bit i = whether `ei` is present); see
//! the `blade` module for named indices. See docs/gac-design.md and
//! [ADR 001](../../../docs/adr/001-geometric-algebra-as-spatial-model.md).

use core::ops::{Add, Mul, Neg, Sub};
use meridian_numeric_core::Scalar;

/// Named blade indices into [`Multivector`]'s 16 components, encoded as a
/// 4-bit mask over `{e0, e1, e2, e3}` (bit i set means `ei` is a factor).
/// Blades are always stored/read in canonical increasing-index order.
pub mod blade {
    pub const S: usize = 0b0000;
    pub const E0: usize = 0b0001;
    pub const E1: usize = 0b0010;
    pub const E2: usize = 0b0100;
    pub const E3: usize = 0b1000;
    pub const E01: usize = 0b0011;
    pub const E02: usize = 0b0101;
    pub const E03: usize = 0b1001;
    pub const E12: usize = 0b0110;
    pub const E13: usize = 0b1010;
    pub const E23: usize = 0b1100;
    pub const E012: usize = 0b0111;
    pub const E013: usize = 0b1011;
    pub const E023: usize = 0b1101;
    pub const E123: usize = 0b1110;
    pub const E0123: usize = 0b1111;
}

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
    fn approx_eq(self, rhs: Vec3) -> bool {
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

/// A named reference frame: origin + basis, expressed as a motor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    pub motor: Motor3,
}

/// A camera/projective mapping: view-space (right-handed, looking down
/// `-Z`, `+X` right, `+Y` up) to clip space. Column-major, column-vector
/// convention (`M * v`), depth range `[0, 1]` — matches wgpu/DX12/Metal,
/// not classic OpenGL's `[-1, 1]`. This is the one hardcoded convention
/// choice here: `graphics-core`'s `Camera` is responsible for turning a
/// `Motor3` world frame (whose local-forward axis is a per-subsystem
/// convention, not a `gac-core` one) into this view space before applying
/// a `Projection` — see docs/graphics-design.md.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection(pub [[Scalar; 4]; 4]);

impl Default for Projection {
    fn default() -> Self {
        let mut m = [[0.0; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        Self(m)
    }
}

impl Projection {
    /// A right-handed perspective projection. `fov_y_radians` is the full
    /// vertical field of view; `aspect` is width/height; `near`/`far` are
    /// positive view-space distances (`0 < near < far`).
    ///
    /// Derivation (standard perspective projection, re-derived here rather
    /// than copied from a specific library so it can be cross-checked
    /// against hand-computed values in tests): a point `(x, y, z)` in view
    /// space (`z` negative in front of the camera) projects to
    /// `x' = x * f / aspect`, `y' = y * f`, where `f = cot(fov_y / 2)`; the
    /// clip-space `w` is `-z`; depth is mapped to `[0, 1]` via
    /// `z' = far * (z + near) / (z * (far - near))` after the perspective
    /// divide, which requires `z_row = [0, 0, far / (near - far), -1]` and
    /// `w_row = [0, 0, near * far / (near - far), 0]` in the matrix below.
    pub fn perspective(fov_y_radians: Scalar, aspect: Scalar, near: Scalar, far: Scalar) -> Self {
        let f = 1.0 / (fov_y_radians * 0.5).tan();
        Self([
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), -1.0],
            [0.0, 0.0, near * far / (near - far), 0.0],
        ])
    }

    /// A right-handed orthographic projection over the view-space box
    /// `[left, right] x [bottom, top] x [-far, -near]`, depth mapped to
    /// `[0, 1]`.
    pub fn orthographic(
        left: Scalar,
        right: Scalar,
        bottom: Scalar,
        top: Scalar,
        near: Scalar,
        far: Scalar,
    ) -> Self {
        Self([
            [2.0 / (right - left), 0.0, 0.0, 0.0],
            [0.0, 2.0 / (top - bottom), 0.0, 0.0],
            [0.0, 0.0, -1.0 / (far - near), 0.0],
            [
                -(right + left) / (right - left),
                -(top + bottom) / (top - bottom),
                -near / (far - near),
                1.0,
            ],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

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
    fn to_mat4_identity_is_the_identity_matrix() {
        let m = Motor3::identity().to_mat4();
        let mut expected = [[0.0; 4]; 4];
        for (i, row) in expected.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        assert_eq!(m, expected);
    }

    /// Independent oracle for `Projection::perspective`: a point placed
    /// exactly on the frustum's side/top planes at a known depth must land
    /// on clip-space's `+-w` boundary (NDC `+-1` after the perspective
    /// divide), computed from plain trigonometry, not the matrix itself.
    #[test]
    fn perspective_projects_frustum_boundary_points_to_ndc_edges() {
        let fov_y = PI / 2.0; // 90 degrees
        let aspect = 16.0 / 9.0;
        let near = 0.1;
        let far = 100.0;
        let proj = Projection::perspective(fov_y, aspect, near, far);

        let depth = 10.0_f32;
        let half_height = depth * (fov_y * 0.5).tan();
        let half_width = half_height * aspect;

        let project = |p: [Scalar; 4]| -> [Scalar; 4] {
            let m = proj.0;
            [
                m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0] * p[3],
                m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1] * p[3],
                m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2] * p[3],
                m[0][3] * p[0] + m[1][3] * p[1] + m[2][3] * p[2] + m[3][3] * p[3],
            ]
        };

        let top_edge = project([0.0, half_height, -depth, 1.0]);
        assert!((top_edge[1] / top_edge[3] - 1.0).abs() < 1e-4);
        let right_edge = project([half_width, 0.0, -depth, 1.0]);
        assert!((right_edge[0] / right_edge[3] - 1.0).abs() < 1e-4);

        let at_near = project([0.0, 0.0, -near, 1.0]);
        assert!(
            (at_near[2] / at_near[3]).abs() < 1e-5,
            "near plane maps to depth 0"
        );
        let at_far = project([0.0, 0.0, -far, 1.0]);
        assert!(
            (at_far[2] / at_far[3] - 1.0).abs() < 1e-5,
            "far plane maps to depth 1"
        );
    }

    #[test]
    fn orthographic_maps_box_corners_to_ndc_cube() {
        let proj = Projection::orthographic(-2.0, 2.0, -1.0, 1.0, 0.5, 10.0);
        let m = proj.0;
        let project = |p: [Scalar; 4]| -> [Scalar; 4] {
            [
                m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0] * p[3],
                m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1] * p[3],
                m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2] * p[3],
                m[0][3] * p[0] + m[1][3] * p[1] + m[2][3] * p[2] + m[3][3] * p[3],
            ]
        };
        // Orthographic w stays 1: no perspective divide needed.
        let near_corner = project([-2.0, -1.0, -0.5, 1.0]);
        assert!((near_corner[0] - -1.0).abs() < 1e-5);
        assert!((near_corner[1] - -1.0).abs() < 1e-5);
        assert!((near_corner[2] - 0.0).abs() < 1e-5);
        let far_corner = project([2.0, 1.0, -10.0, 1.0]);
        assert!((far_corner[0] - 1.0).abs() < 1e-5);
        assert!((far_corner[1] - 1.0).abs() < 1e-5);
        assert!((far_corner[2] - 1.0).abs() < 1e-5);
    }
}
