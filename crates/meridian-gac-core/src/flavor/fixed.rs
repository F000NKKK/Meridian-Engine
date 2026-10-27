//! The deterministic `Fixed`-backed flavor.

use meridian_numeric_core::Fixed;

use super::{GaFlavor, ScalarLike};
use crate::bivector::fixed::FixedBivector3;
use crate::motor::fixed::FixedMotor3;
use crate::rotor::fixed::FixedRotor;
use crate::vector::fixed::FixedVec3;

/// A small tolerance a few Q16.16 steps above zero, used the same way
/// [`meridian_numeric_core::EPSILON`] is used in [`crate::float_ga`] —
/// degenerate (near-zero-length) vector/bivector guards.
pub(crate) const FIXED_EPSILON: Fixed = Fixed::from_bits(4); // 4 / 65536 ~= 6.1e-5

/// The deterministic GA flavor: `Fixed`-backed. See the crate root doc
/// comment for what "flavor" means here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FixedFlavor;

impl ScalarLike for Fixed {
    const ZERO: Self = Fixed::ZERO;
    const ONE: Self = Fixed::ONE;
    const EPSILON: Self = FIXED_EPSILON;
    const MAX: Self = Fixed::MAX;

    fn from_f64(v: f64) -> Self {
        Fixed::from_num(v)
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

impl GaFlavor for FixedFlavor {
    type Scalar = Fixed;
    type Vector = FixedVec3;
    type Bivector = FixedBivector3;
    type Rotor = FixedRotor;
    type Motor = FixedMotor3;
}
