//! Checked floating-point rotation construction.

use super::RotationError;
use crate::{Rotor, Vec3};

impl Rotor {
    /// Right-handed rotation about a nonzero axis, in radians.
    /// Finite axes need not be normalized, even at very large or small magnitudes.
    ///
    /// # Errors
    /// Returns [`RotationError`] for zero axes or non-finite inputs.
    pub fn try_from_axis_angle(axis: Vec3, radians: f32) -> Result<Self, RotationError> {
        if !radians.is_finite() {
            return Err(RotationError::NonFinite);
        }
        Ok(Self::from_axis_angle(unit(axis)?, radians))
    }

    /// Shortest rotation mapping `from` onto `to`, preserving direction length.
    /// Opposite directions use a deterministic perpendicular axis (the least
    /// aligned coordinate axis crossed with `from`); their roll is not unique.
    ///
    /// # Errors
    /// Returns [`RotationError`] for zero or non-finite directions.
    pub fn between_directions(from: Vec3, to: Vec3) -> Result<Self, RotationError> {
        let from = unit(from)?;
        let to = unit(to)?;
        let cross = from.cross(to);
        let cosine = from.dot(to).clamp(-1.0, 1.0);
        if cross.length_squared() > 0.0 {
            let angle = cross.length().atan2(cosine);
            return Self::try_from_axis_angle(cross, angle);
        }
        if cosine >= 0.0 {
            return Ok(Self::identity());
        }
        let absolute = [from.x.abs(), from.y.abs(), from.z.abs()];
        let basis = if absolute[0] <= absolute[1] && absolute[0] <= absolute[2] {
            Vec3::X
        } else if absolute[1] <= absolute[2] {
            Vec3::Y
        } else {
            Vec3::Z
        };
        Self::try_from_axis_angle(from.cross(basis), core::f32::consts::PI)
    }
}

fn unit(vector: Vec3) -> Result<Vec3, RotationError> {
    let values = [vector.x, vector.y, vector.z];
    if !values.iter().all(|value| value.is_finite()) {
        return Err(RotationError::NonFinite);
    }
    let maximum = values.iter().map(|value| value.abs()).fold(0.0, f32::max);
    if maximum == 0.0 {
        return Err(RotationError::ZeroDirection);
    }
    let scaled = Vec3::new(vector.x / maximum, vector.y / maximum, vector.z / maximum);
    Ok(scaled * (1.0 / scaled.length()))
}
