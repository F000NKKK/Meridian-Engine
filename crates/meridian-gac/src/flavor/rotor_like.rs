//! The rotor abstraction generic code is written against.

use super::{ScalarLike, VectorLike};

/// A pure-rotation type usable generically — implemented for
/// [`crate::Rotor`] and [`crate::FixedRotor`].
pub trait RotorLike: Copy + core::fmt::Debug {
    /// Scalar coefficient type.
    type Scalar: ScalarLike;
    /// Vector type rotated by this rotor.
    type Vector: VectorLike<Scalar = Self::Scalar>;

    /// Returns the identity rotation.
    fn identity() -> Self;
    /// Constructs a rotation around `axis` by `angle` radians.
    fn from_axis_angle(axis: Self::Vector, angle: Self::Scalar) -> Self;
    /// Applies this rotation first and `rhs` second.
    fn compose(self, rhs: Self) -> Self;
    /// Reverses this unit rotation.
    fn reverse(self) -> Self;
    /// Rotates a direction vector.
    fn transform_vector(self, v: Self::Vector) -> Self::Vector;
}
