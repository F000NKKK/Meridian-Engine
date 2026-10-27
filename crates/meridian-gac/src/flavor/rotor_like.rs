//! The rotor abstraction generic code is written against.

use super::{ScalarLike, VectorLike};

/// A pure-rotation type usable generically — implemented for
/// [`crate::Rotor`] and [`crate::FixedRotor`].
pub trait RotorLike: Copy + core::fmt::Debug {
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;

    fn identity() -> Self;
    fn from_axis_angle(axis: Self::Vector, angle: Self::Scalar) -> Self;
    fn compose(self, rhs: Self) -> Self;
    fn reverse(self) -> Self;
    fn transform_vector(self, v: Self::Vector) -> Self::Vector;
}
