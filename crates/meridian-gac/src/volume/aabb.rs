//! Axis-aligned bounding box.

use super::Shape;
use crate::flavor::{GaFlavor, ScalarLike, VectorLike};

/// An axis-aligned bounding box: plain spatial-extent math with no domain
/// meaning of its own, shared by every subsystem that needs a cheap
/// overlap/culling test (`physics-core`'s broad phase, `graphics-core`'s
/// frustum culling, ...). The axis-aligned of the two box variants — see
/// [`Obb`](crate::Obb) for the oriented one; a cube is either with equal extents on
/// every axis, not a separate type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb<F: GaFlavor> {
    pub min: F::Vector,
    pub max: F::Vector,
}

impl<F: GaFlavor> Default for Aabb<F> {
    fn default() -> Self {
        Self {
            min: F::Vector::ZERO,
            max: F::Vector::ZERO,
        }
    }
}

impl<F: GaFlavor> Aabb<F> {
    pub fn from_sphere(center: F::Vector, radius: F::Scalar) -> Self {
        let r = F::Vector::new(radius, radius, radius);
        Self {
            min: center - r,
            max: center + r,
        }
    }

    /// An axis-aligned cube: equal half-extent on every axis.
    pub fn cube(center: F::Vector, half_extent: F::Scalar) -> Self {
        Self::from_sphere(center, half_extent)
    }

    pub fn overlaps(&self, other: &Aabb<F>) -> bool {
        self.min.x() <= other.max.x()
            && self.max.x() >= other.min.x()
            && self.min.y() <= other.max.y()
            && self.max.y() >= other.min.y()
            && self.min.z() <= other.max.z()
            && self.max.z() >= other.min.z()
    }
}

impl<F: GaFlavor> Shape<F> for Aabb<F> {
    /// Picks the corner furthest along `direction` per axis independently
    /// — this is the exact "positive vertex" trick frustum/AABB tests use
    /// by hand; expressing it as a support function is what lets the same
    /// trick generalize to [`ConvexVolume::intersects`](crate::ConvexVolume::intersects) for every shape.
    fn support(&self, direction: F::Vector) -> F::Vector {
        F::Vector::new(
            if direction.x() >= F::Scalar::ZERO {
                self.max.x()
            } else {
                self.min.x()
            },
            if direction.y() >= F::Scalar::ZERO {
                self.max.y()
            } else {
                self.min.y()
            },
            if direction.z() >= F::Scalar::ZERO {
                self.max.z()
            } else {
                self.min.z()
            },
        )
    }
}
