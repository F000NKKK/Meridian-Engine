//! Sphere.

use super::Shape;
use crate::flavor::{GaFlavor, ScalarLike, VectorLike};

/// A sphere: center + radius. The simplest curved primitive, and the one
/// most subsystems reach for first — `physics-domain`'s only default
/// collider shape, a natural bounding volume for `rendering-domain` culling
/// later.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere<F: GaFlavor> {
    pub center: F::Vector,
    pub radius: F::Scalar,
}

impl<F: GaFlavor> Default for Sphere<F> {
    fn default() -> Self {
        Self {
            center: F::Vector::ZERO,
            radius: F::Scalar::ZERO,
        }
    }
}

impl<F: GaFlavor> Shape<F> for Sphere<F> {
    fn support(&self, direction: F::Vector) -> F::Vector {
        self.center + direction.normalize() * self.radius
    }
}
