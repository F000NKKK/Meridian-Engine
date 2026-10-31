//! Orientation from a forward direction and an up hint.

use super::{RotationError, checked::unit};
use crate::{Rotor, Vec3};

impl Rotor {
    /// Maps local +Z to `forward` and local +Y towards the projected `up` hint.
    /// Right-handed; the up hint need not be perpendicular or normalized.
    /// Camera view conventions (often local -Z) require an explicit adjustment.
    ///
    /// # Errors
    /// Rejects zero/non-finite directions and normalized forward/up pairs whose
    /// cross-product length is at most 1e-6 (roll is numerically undefined).
    pub fn look_rotation(forward: Vec3, up: Vec3) -> Result<Self, RotationError> {
        let forward = unit(forward)?;
        let up = unit(up)?;
        let right = up.cross(forward);
        if right.length() <= 1e-6 {
            return Err(RotationError::ParallelUp);
        }
        let desired_up = forward.cross(unit(right)?);
        let aligned = Self::between_directions(Vec3::Z, forward)?;
        let current_up = aligned.transform_vector(Vec3::Y);
        let sine = forward.dot(current_up.cross(desired_up));
        let cosine = current_up.dot(desired_up).clamp(-1.0, 1.0);
        let roll = Self::try_from_axis_angle(forward, sine.atan2(cosine))?;
        aligned.compose(roll).try_normalized()
    }
}
