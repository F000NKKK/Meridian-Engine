//! Oriented bounding box.

use super::Shape;
use crate::flavor::{GaFlavor, MotorLike, ScalarLike, VectorLike};

/// An oriented (rotated) box — the other of the two box variants; see
/// [`Aabb`](crate::Aabb) for the axis-aligned one. A cube is either with equal
/// `half_extents` on every axis, not a separate type. Position *and*
/// orientation are one `frame: F::Motor`, not a separate center/rotation
/// pair — the same convention every rigid pose in the workspace uses
/// (`RigidBody`, `Camera`, `Listener`/`Emitter`; see docs/gac-design.md),
/// so a physics `RigidBody`'s own `frame` can be used here directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb<F: GaFlavor> {
    pub frame: F::Motor,
    pub half_extents: F::Vector,
}

impl<F: GaFlavor> Default for Obb<F> {
    fn default() -> Self {
        Self {
            frame: F::Motor::identity(),
            half_extents: F::Vector::ZERO,
        }
    }
}

impl<F: GaFlavor> Obb<F> {
    /// An oriented cube: equal half-extent on every axis.
    pub fn cube(frame: F::Motor, half_extent: F::Scalar) -> Self {
        Self {
            frame,
            half_extents: F::Vector::new(half_extent, half_extent, half_extent),
        }
    }
}

impl<F: GaFlavor> Shape<F> for Obb<F> {
    fn support(&self, direction: F::Vector) -> F::Vector {
        // Rotate the query direction into the box's local (axis-aligned)
        // space, pick the local corner, then carry that corner back out
        // to world space through the full frame (rotation *and*
        // translation) — the same idea as Aabb::support, done in a frame
        // where the box actually is axis-aligned.
        let local_direction = self.frame.inverse().transform_vector(direction);
        let local_support = F::Vector::new(
            self.half_extents.x() * local_direction.x().signum(),
            self.half_extents.y() * local_direction.y().signum(),
            self.half_extents.z() * local_direction.z().signum(),
        );
        self.frame.transform_point(local_support)
    }
}
