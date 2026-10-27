//! Unit axes of a flavor's vector type.

use meridian_gac::{GaFlavor, ScalarLike, VectorLike};

pub(crate) fn axis_x<F: GaFlavor>() -> F::Vector {
    F::Vector::new(F::Scalar::ONE, F::Scalar::ZERO, F::Scalar::ZERO)
}

pub(crate) fn axis_y<F: GaFlavor>() -> F::Vector {
    F::Vector::new(F::Scalar::ZERO, F::Scalar::ONE, F::Scalar::ZERO)
}

pub(crate) fn axis_z<F: GaFlavor>() -> F::Vector {
    F::Vector::new(F::Scalar::ZERO, F::Scalar::ZERO, F::Scalar::ONE)
}
