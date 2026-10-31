//! Validation at raw rotation and interpolation boundaries.

use super::RotationError;
use crate::{Rotor, blade};

impl Rotor {
    /// Validates raw coefficients and returns a unit rotor, preserving its sign.
    /// Accepts finite scalar/e12/e13/e23 coefficients only; zero norm is invalid.
    /// Existing unchecked methods do not call this automatically.
    ///
    /// # Errors
    /// Rejects non-finite coefficients, non-rotation blades and zero rotors.
    pub fn try_normalized(self) -> Result<Self, RotationError> {
        for (index, value) in self.0.0.iter().enumerate() {
            if !value.is_finite() {
                return Err(RotationError::NonFinite);
            }
            if ![blade::S, blade::E12, blade::E13, blade::E23].contains(&index) && *value != 0.0 {
                return Err(RotationError::InvalidRotor);
            }
        }
        let quaternion = self.to_quaternion();
        let maximum = quaternion.iter().map(|v| v.abs()).fold(0.0, f32::max);
        if maximum == 0.0 {
            return Err(RotationError::InvalidRotor);
        }
        Ok(Self::from_quaternion(quaternion.map(|v| v / maximum)))
    }

    /// Interpolates validated, normalized endpoints along the shortest arc.
    ///
    /// # Errors
    /// Rejects invalid rotors and non-finite or out-of-range fractions.
    pub fn try_slerp(self, other: Self, fraction: f32) -> Result<Self, RotationError> {
        if !fraction.is_finite() {
            return Err(RotationError::NonFinite);
        }
        if !(0.0..=1.0).contains(&fraction) {
            return Err(RotationError::InvalidFraction);
        }
        Ok(self
            .try_normalized()?
            .slerp(other.try_normalized()?, fraction))
    }
}
