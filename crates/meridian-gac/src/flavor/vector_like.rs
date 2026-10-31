//! The vector abstraction generic code is written against.

use core::ops::{Add, Mul, Neg, Sub};

use super::ScalarLike;

/// A 3D vector type usable generically — implemented for
/// [`crate::Vec3`] and [`crate::FixedVec3`]. See [`ScalarLike`]'s
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
    /// Scalar coefficient type.
    type Scalar: ScalarLike;

    /// The zero vector.
    const ZERO: Self;

    /// Constructs a vector from its three coordinates.
    fn new(x: Self::Scalar, y: Self::Scalar, z: Self::Scalar) -> Self;
    /// Returns the X coordinate.
    fn x(self) -> Self::Scalar;
    /// Returns the Y coordinate.
    fn y(self) -> Self::Scalar;
    /// Returns the Z coordinate.
    fn z(self) -> Self::Scalar;
    /// Computes the dot product with `rhs`.
    fn dot(self, rhs: Self) -> Self::Scalar;
    /// Computes the cross product with `rhs`.
    fn cross(self, rhs: Self) -> Self;
    /// Returns the Euclidean length.
    fn length(self) -> Self::Scalar;
    /// Returns the squared Euclidean length.
    fn length_squared(self) -> Self::Scalar;
    /// Returns a unit-length vector in the same direction.
    fn normalize(self) -> Self;
}
