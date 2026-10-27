//! The motor abstraction generic code is written against.

use super::{RotorLike, ScalarLike, VectorLike};

/// A rigid-motion (rotation + translation) type usable generically —
/// implemented for [`crate::Motor3`] and [`crate::FixedMotor3`].
pub trait MotorLike: Copy + core::fmt::Debug {
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;

    fn identity() -> Self;
    fn translation(t: Self::Vector) -> Self;
    fn from_rotation_translation(rotor: Self::Rotor, t: Self::Vector) -> Self;
    fn compose(self, rhs: Self) -> Self;
    fn inverse(self) -> Self;
    fn transform_point(self, p: Self::Vector) -> Self::Vector;
    fn transform_vector(self, v: Self::Vector) -> Self::Vector;
}
