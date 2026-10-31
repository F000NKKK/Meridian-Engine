//! The fixed-point (`Fixed`) rotor, mirroring [`super::float`].

use meridian_numeric::Fixed;

use super::float::Rotor;
use crate::blade;
use crate::flavor::RotorLike;
use crate::multivector::fixed::FixedMultivector;
use crate::multivector::float::Multivector;
use crate::vector::fixed::FixedVec3;

/// Mirrors [`crate::Rotor`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedRotor(pub FixedMultivector);

impl Default for FixedRotor {
    fn default() -> Self {
        FixedRotor::identity()
    }
}

impl FixedRotor {
    /// Returns the identity rotation.
    pub fn identity() -> Self {
        FixedRotor(FixedMultivector::scalar(Fixed::ONE))
    }

    /// Constructs a rotation around `axis` by `angle` radians.
    pub fn from_axis_angle(axis: FixedVec3, angle: Fixed) -> Self {
        let axis = axis.normalize();
        let half = angle * Fixed::from_num(0.5);
        let (s, c) = half.sin_cos();
        let mut m = [Fixed::ZERO; 16];
        m[blade::S] = c;
        m[blade::E23] = -s * axis.x;
        m[blade::E13] = s * axis.y;
        m[blade::E12] = -s * axis.z;
        FixedRotor(FixedMultivector(m))
    }

    /// Applies this rotation first and `rhs` second.
    pub fn compose(self, rhs: FixedRotor) -> FixedRotor {
        FixedRotor(rhs.0 * self.0)
    }

    /// Reverses this unit rotation.
    pub fn reverse(self) -> FixedRotor {
        FixedRotor(self.0.reverse())
    }

    /// Rotates a direction vector.
    pub fn transform_vector(self, v: FixedVec3) -> FixedVec3 {
        let mut vm = [Fixed::ZERO; 16];
        vm[blade::E1] = v.x;
        vm[blade::E2] = v.y;
        vm[blade::E3] = v.z;
        let p = FixedMultivector(vm);
        let r = self.0 * p * self.0.reverse();
        FixedVec3::new(r.0[blade::E1], r.0[blade::E2], r.0[blade::E3])
    }
}

impl RotorLike for FixedRotor {
    type Scalar = Fixed;
    type Vector = FixedVec3;

    fn identity() -> Self {
        FixedRotor::identity()
    }
    fn from_axis_angle(axis: FixedVec3, angle: Fixed) -> Self {
        FixedRotor::from_axis_angle(axis, angle)
    }
    fn compose(self, rhs: Self) -> Self {
        self.compose(rhs)
    }
    fn reverse(self) -> Self {
        self.reverse()
    }
    fn transform_vector(self, v: FixedVec3) -> FixedVec3 {
        self.transform_vector(v)
    }
}

impl FixedRotor {
    /// Deliberate precision-changing cast to the default flavor —
    /// component-wise across the underlying multivector, not a
    /// re-derivation through axis/angle.
    pub fn to_float_lossy(self) -> Rotor {
        let mut out = [0.0f32; 16];
        for (dst, src) in out.iter_mut().zip(self.0.0) {
            *dst = src.to_num() as f32;
        }
        Rotor(Multivector(out))
    }
}
