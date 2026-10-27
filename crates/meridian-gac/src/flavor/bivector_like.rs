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
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;

    const ZERO: Self;

    fn wedge(a: Self::Vector, b: Self::Vector) -> Self;
    fn length(self) -> Self::Scalar;
    fn exp(self) -> Self::Rotor;
}
