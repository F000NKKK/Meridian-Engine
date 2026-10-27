//! The vector abstraction generic code is written against.

use core::ops::{Add, Mul, Neg, Sub};

use super::ScalarLike;

/// A 3D vector type usable generically — implemented for
/// [`crate::float_ga::Vec3`] and [`crate::fixed_ga::FixedVec3`]. See [`ScalarLike`]'s
/// doc comment for why every method is a thin forward, not a
/// reimplementation.
pub trait VectorLike:
    Copy
    + core::fmt::Debug
    + PartialEq
    + Add<Output = Self>
    + Sub<Output = Self>
    + Neg<Output = Self>
    + Mul<<Self as VectorLike>::Scalar, Output = Self>
{
    type Scalar: ScalarLike;

    const ZERO: Self;

    fn new(x: Self::Scalar, y: Self::Scalar, z: Self::Scalar) -> Self;
    fn x(self) -> Self::Scalar;
    fn y(self) -> Self::Scalar;
    fn z(self) -> Self::Scalar;
    fn dot(self, rhs: Self) -> Self::Scalar;
    fn cross(self, rhs: Self) -> Self;
    fn length(self) -> Self::Scalar;
    fn length_squared(self) -> Self::Scalar;
    fn normalize(self) -> Self;
}
