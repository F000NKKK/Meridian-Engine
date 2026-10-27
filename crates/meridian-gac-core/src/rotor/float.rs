//! The float (`f32`) rotor: a pure rotation.

use meridian_numeric_core::{Fixed, Scalar};

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

    /// Composes two rotations: `self` applied first, then `rhs` — i.e.
    /// `rhs.compose(self)` in row-vector convention, `self` in column-
    /// vector/geometric-product convention. Matches [`Motor3::compose`](crate::Motor3::compose).
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
