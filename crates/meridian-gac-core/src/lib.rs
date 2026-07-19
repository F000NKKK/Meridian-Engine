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
}

/// A named reference frame: origin + basis, expressed as a motor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    pub motor: Motor3,
}

/// A camera/projective mapping.
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
}
