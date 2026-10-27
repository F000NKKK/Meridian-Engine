//! The bundle of associated types that makes one complete algebra flavor.

use super::{BivectorLike, MotorLike, RotorLike, ScalarLike, VectorLike};

/// One complete "flavor" of the algebra — the associated-type bundle
/// that lets generic code (this crate's own primitives below,
/// `physics-core`'s engine, ...) be written once against `F: GaFlavor`
/// instead of once per scalar type. [`crate::float_ga::FloatFlavor`] and
/// [`crate::fixed_ga::FixedFlavor`] are the two flavors that exist today; a
/// third is exactly as easy to add as implementing this trait (plus the
/// five component traits above) for a new scalar/vector/bivector/rotor/
/// motor set.
pub trait GaFlavor: Copy + Default + core::fmt::Debug + PartialEq + 'static {
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;
    type Bivector: BivectorLike<Scalar = Self::Scalar, Vector = Self::Vector, Rotor = Self::Rotor>;
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;
    type Motor: MotorLike<Scalar = Self::Scalar, Vector = Self::Vector, Rotor = Self::Rotor>;
}
