//! The motor abstraction generic code is written against.

use super::{RotorLike, ScalarLike, VectorLike};

/// A rigid-motion (rotation + translation) type usable generically —
/// implemented for [`crate::Motor3`] and [`crate::FixedMotor3`].
pub trait MotorLike: Copy + core::fmt::Debug {
    /// Scalar coefficient type.
    type Scalar: ScalarLike;
    /// Point and direction vector type.
    type Vector: VectorLike<Scalar = Self::Scalar>;
    /// Pure rotation type.
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;

    /// Returns the identity rigid motion.
    fn identity() -> Self;
    /// Constructs a pure translation.
    fn translation(t: Self::Vector) -> Self;
    /// Constructs a rotation followed by a translation.
    fn from_rotation_translation(rotor: Self::Rotor, t: Self::Vector) -> Self;
    /// Applies this motion first and `rhs` second.
    fn compose(self, rhs: Self) -> Self;
    /// Returns the inverse rigid motion.
    fn inverse(self) -> Self;
    /// Transforms a point, including translation.
    fn transform_point(self, p: Self::Vector) -> Self::Vector;
    /// Transforms a direction without translation.
    fn transform_vector(self, v: Self::Vector) -> Self::Vector;
}
