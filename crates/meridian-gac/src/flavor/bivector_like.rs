//! The bivector abstraction generic code is written against.

use core::ops::{Add, Mul, Sub};

use super::{RotorLike, ScalarLike, VectorLike};

/// A rotation-generator bivector type usable generically —
/// implemented for [`crate::Bivector3`] and
/// [`crate::FixedBivector3`].
pub trait BivectorLike:
    Copy
    + core::fmt::Debug
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<<Self as BivectorLike>::Scalar, Output = Self>
{
    /// Scalar coefficient type.
    type Scalar: ScalarLike;
    /// Vector type used by wedge products.
    type Vector: VectorLike<Scalar = Self::Scalar>;
    /// Rotor produced by the exponential map.
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;

    /// The zero bivector.
    const ZERO: Self;

    /// Computes the wedge product of two vectors.
    fn wedge(a: Self::Vector, b: Self::Vector) -> Self;
    /// Returns the bivector's magnitude.
    fn length(self) -> Self::Scalar;
    /// Maps this rotation generator to a rotor.
    fn exp(self) -> Self::Rotor;
}
