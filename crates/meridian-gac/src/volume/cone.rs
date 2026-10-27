//! Cone.

use super::Shape;
use crate::flavor::{GaFlavor, ScalarLike, VectorLike};

/// A right circular cone: apex at `apex`, opening toward `apex + axis *
/// height` (`axis` must be unit length), `half_angle` (radians) between
/// the axis and the cone's slanted surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone<F: GaFlavor> {
    pub apex: F::Vector,
    pub axis: F::Vector,
    pub half_angle: F::Scalar,
    pub height: F::Scalar,
}

impl<F: GaFlavor> Shape<F> for Cone<F> {
    /// A cone's convex hull is its apex plus its base disk's rim; the
    /// support point is whichever of {apex, the rim point closest to
    /// `direction`} is farther along `direction`. The rim point is found
    /// by projecting `direction` onto the base plane (perpendicular to
    /// `axis`) and walking out to the base radius in that direction —
    /// exactly how a disk's own support function works.
    fn support(&self, direction: F::Vector) -> F::Vector {
        let base_center = self.apex + self.axis * self.height;
        let base_radius = self.height * self.half_angle.tan();

        let along_axis = direction.dot(self.axis);
        let perpendicular = direction - self.axis * along_axis;
        let perpendicular_len = perpendicular.length();

        let rim_point = if perpendicular_len > F::Scalar::EPSILON {
            base_center + perpendicular * (base_radius / perpendicular_len)
        } else {
            base_center
        };

        if direction.dot(self.apex) >= direction.dot(rim_point) {
            self.apex
        } else {
            rim_point
        }
    }
}
