//! The fixed-point (`Fixed`) bivector, mirroring [`super::float`].

use core::ops::{Add, Mul, Sub};

use meridian_numeric_core::Fixed;

use super::float::Bivector3;
use crate::flavor::BivectorLike;
use crate::flavor::fixed::FIXED_EPSILON;
use crate::rotor::fixed::FixedRotor;
use crate::vector::fixed::FixedVec3;

/// Mirrors [`crate::float_ga::Bivector3`] — see that type's doc comment
/// for why angular quantities are bivectors, not vectors.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FixedBivector3 {
    pub e23: Fixed,
    pub e31: Fixed,
    pub e12: Fixed,
}

impl FixedBivector3 {
    pub const ZERO: FixedBivector3 = FixedBivector3 {
        e23: Fixed::ZERO,
        e31: Fixed::ZERO,
        e12: Fixed::ZERO,
    };

    pub const fn new(e23: Fixed, e31: Fixed, e12: Fixed) -> Self {
        Self { e23, e31, e12 }
    }

    pub fn wedge(a: FixedVec3, b: FixedVec3) -> FixedBivector3 {
        FixedBivector3::new(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x,
        )
    }

    pub fn length(self) -> Fixed {
        (self.e23 * self.e23 + self.e31 * self.e31 + self.e12 * self.e12).sqrt()
    }

    pub fn exp(self) -> FixedRotor {
        let angle = self.length();
        if angle <= FIXED_EPSILON {
            return FixedRotor::identity();
        }
        let axis = FixedVec3::new(self.e23, self.e31, self.e12) * (Fixed::ONE / angle);
        FixedRotor::from_axis_angle(axis, angle)
    }
}

impl Add for FixedBivector3 {
    type Output = FixedBivector3;
    fn add(self, rhs: FixedBivector3) -> FixedBivector3 {
        FixedBivector3::new(self.e23 + rhs.e23, self.e31 + rhs.e31, self.e12 + rhs.e12)
    }
}

impl Sub for FixedBivector3 {
    type Output = FixedBivector3;
    fn sub(self, rhs: FixedBivector3) -> FixedBivector3 {
        FixedBivector3::new(self.e23 - rhs.e23, self.e31 - rhs.e31, self.e12 - rhs.e12)
    }
}

impl Mul<Fixed> for FixedBivector3 {
    type Output = FixedBivector3;
    fn mul(self, rhs: Fixed) -> FixedBivector3 {
        FixedBivector3::new(self.e23 * rhs, self.e31 * rhs, self.e12 * rhs)
    }
}

impl BivectorLike for FixedBivector3 {
    type Scalar = Fixed;
    type Vector = FixedVec3;
    type Rotor = FixedRotor;

    const ZERO: Self = FixedBivector3::ZERO;

    fn wedge(a: FixedVec3, b: FixedVec3) -> Self {
        FixedBivector3::wedge(a, b)
    }
    fn length(self) -> Fixed {
        self.length()
    }
    fn exp(self) -> FixedRotor {
        self.exp()
    }
}

impl FixedBivector3 {
    /// Deliberate precision-changing cast to the default flavor.
    pub fn to_float_lossy(self) -> Bivector3 {
        Bivector3::new(
            self.e23.to_num() as f32,
            self.e31.to_num() as f32,
            self.e12.to_num() as f32,
        )
    }
}
