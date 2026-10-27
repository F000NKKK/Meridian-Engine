//! The default `f32`-backed flavor, GPU-dispatchable.

use meridian_numeric_core::Scalar;

use super::{GaFlavor, ScalarLike};
use crate::bivector::float::Bivector3;
use crate::motor::float::Motor3;
use crate::rotor::float::Rotor;
use crate::vector::float::Vec3;

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

impl GaFlavor for FloatFlavor {
    type Scalar = Scalar;
    type Vector = Vec3;
    type Bivector = Bivector3;
    type Rotor = Rotor;
    type Motor = Motor3;
}
