//! The bundle of associated types that makes one complete algebra flavor.

use super::{BivectorLike, MotorLike, RotorLike, ScalarLike, VectorLike};

/// One complete "flavor" of the algebra — the associated-type bundle
/// that lets generic code (this crate's own primitives below,
/// `physics-domain`'s engine, ...) be written once against `F: GaFlavor`
/// instead of once per scalar type. [`crate::FloatFlavor`] and
/// [`crate::FixedFlavor`] are the two flavors that exist today; a
/// third is exactly as easy to add as implementing this trait (plus the
/// five component traits above) for a new scalar/vector/bivector/rotor/
/// motor set.
pub trait GaFlavor: Copy + Default + core::fmt::Debug + PartialEq + 'static {
    /// Scalar coefficient type for this flavor.
    type Scalar: ScalarLike;
    /// Three-dimensional vector type.
    type Vector: VectorLike<Scalar = Self::Scalar>;
    /// Rotation-generator bivector type.
    type Bivector: BivectorLike<Scalar = Self::Scalar, Vector = Self::Vector, Rotor = Self::Rotor>;
    /// Pure rotation type.
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;
    /// Rigid rotation-and-translation type.
    type Motor: MotorLike<Scalar = Self::Scalar, Vector = Self::Vector, Rotor = Self::Rotor>;
}
