//! The float (`f32`) rotor: a pure rotation.

use meridian_numeric::{Fixed, Scalar};

use super::fixed::FixedRotor;
use crate::blade;
use crate::flavor::RotorLike;
use crate::multivector::fixed::FixedMultivector;
use crate::multivector::float::Multivector;
use crate::vector::float::Vec3;

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

    /// The rotation of the unit quaternion `[x, y, z, w]` (the order glTF
    /// stores them in; `w` is the scalar part). The input is normalised; a
    /// zero quaternion has no direction and gives the identity.
    #[must_use]
    pub fn from_quaternion(quaternion: [Scalar; 4]) -> Self {
        let length = quaternion.iter().map(|c| c * c).sum::<Scalar>().sqrt();
        if length <= Scalar::EPSILON {
            return Rotor::identity();
        }
        let mut m = [0.0; 16];
        m[blade::S] = quaternion[3] / length;
        m[blade::E23] = -quaternion[0] / length;
        m[blade::E13] = quaternion[1] / length;
        m[blade::E12] = -quaternion[2] / length;
        Rotor(Multivector(m))
    }

    /// The unit quaternion `[x, y, z, w]` of this rotation, the inverse of
    /// [`from_quaternion`](Self::from_quaternion).
    #[must_use]
    pub fn to_quaternion(self) -> [Scalar; 4] {
        let m = self.0.0;
        [-m[blade::E23], m[blade::E13], -m[blade::E12], m[blade::S]]
    }

    /// The rotation `t` of the way from `self` to `other` (`t = 0` is `self`,
    /// `t = 1` is `other`) along the shorter arc at constant angular speed.
    /// Nearly parallel rotations are blended linearly, which is accurate
    /// there and avoids dividing by a vanishing sine.
    #[must_use]
    pub fn slerp(self, other: Rotor, t: Scalar) -> Rotor {
        let a = self.to_quaternion();
        let mut b = other.to_quaternion();
        let mut cosine: Scalar = a.iter().zip(&b).map(|(p, q)| p * q).sum();
        if cosine < 0.0 {
            b = b.map(|v| -v);
            cosine = -cosine;
        }
        let (from, to) = if cosine > 0.9995 {
            (1.0 - t, t)
        } else {
            let angle = cosine.acos();
            let sine = angle.sin();
            (((1.0 - t) * angle).sin() / sine, (t * angle).sin() / sine)
        };
        Rotor::from_quaternion(std::array::from_fn(|i| a[i] * from + b[i] * to))
    }

    /// Applies `self` first, then `rhs`:
    /// `a.compose(b).transform_vector(v) == b.transform_vector(a.transform_vector(v))`.
    /// The stored geometric product is `b * a`; this is application order,
    /// not the textual order of matrix multiplication.
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
