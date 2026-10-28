//! [`Scale3`]: a per-axis scale that is valid by construction.

use core::fmt;

use meridian_numeric::Scalar;

use crate::Vec3;

/// Why an axis was refused as a scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleError {
    /// The axis (0 = x, 1 = y, 2 = z).
    pub axis: usize,
    /// The refused value: zero, negative, NaN or infinite.
    pub value: Scalar,
}

impl fmt::Display for ScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scale[{}] = {} is not a finite number above zero",
            self.axis, self.value
        )
    }
}

impl core::error::Error for ScaleError {}

/// A scale along the local axes of an entity, applied before its motor
/// (`model = motor * diag(scale)`): no shear, no reflection. Every axis is
/// finite and above zero, so a scaled mesh keeps its face winding and never
/// collapses to a plane. The default is the unit scale.
///
/// A reflection is not a motor and flips the winding of a mesh's faces, so
/// it is refused here and may be allowed by a later, separate decision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale3 {
    x: Scalar,
    y: Scalar,
    z: Scalar,
}

impl Scale3 {
    /// The scale that changes nothing.
    pub const UNIT: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };

    /// A scale of `x`, `y` and `z` along the local axes.
    ///
    /// # Errors
    ///
    /// [`ScaleError`] naming the first axis that is zero, negative, NaN or infinite.
    pub fn new(x: Scalar, y: Scalar, z: Scalar) -> Result<Self, ScaleError> {
        for (axis, value) in [x, y, z].into_iter().enumerate() {
            if !(value.is_finite() && value > 0.0) {
                return Err(ScaleError { axis, value });
            }
        }
        Ok(Self { x, y, z })
    }

    /// The same scale on all three axes.
    ///
    /// # Errors
    ///
    /// [`ScaleError`] when `factor` is zero, negative, NaN or infinite.
    pub fn uniform(factor: Scalar) -> Result<Self, ScaleError> {
        Self::new(factor, factor, factor)
    }

    /// The three factors, in axis order.
    #[must_use]
    pub const fn components(self) -> [Scalar; 3] {
        [self.x, self.y, self.z]
    }

    /// Whether this is the unit scale.
    #[must_use]
    pub fn is_unit(self) -> bool {
        self == Self::UNIT
    }

    /// `vector` with each component multiplied by its axis factor.
    #[must_use]
    pub fn apply(self, vector: Vec3) -> Vec3 {
        Vec3::new(vector.x * self.x, vector.y * self.y, vector.z * self.z)
    }

    /// `vector` with each component divided by its axis factor: the inverse
    /// of [`apply`](Self::apply).
    #[must_use]
    pub fn unapply(self, vector: Vec3) -> Vec3 {
        Vec3::new(vector.x / self.x, vector.y / self.y, vector.z / self.z)
    }
}

impl Default for Scale3 {
    fn default() -> Self {
        Self::UNIT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scale_applies_per_axis_and_unit_changes_nothing() {
        let scale = Scale3::new(2.0, 3.0, 0.5).unwrap();
        assert_eq!(
            scale.apply(Vec3::new(1.0, 1.0, 2.0)),
            Vec3::new(2.0, 3.0, 1.0)
        );
        assert_eq!(
            scale.unapply(scale.apply(Vec3::new(1.0, 2.0, 3.0))),
            Vec3::new(1.0, 2.0, 3.0)
        );
        assert!(Scale3::default().is_unit());
        assert!(!scale.is_unit());
        assert_eq!(Scale3::uniform(2.0).unwrap().components(), [2.0; 3]);
    }

    #[test]
    fn zero_negative_and_non_finite_axes_are_refused_with_their_axis() {
        for (bad, axis) in [
            (Scale3::new(0.0, 1.0, 1.0), 0),
            (Scale3::new(1.0, -1.0, 1.0), 1),
            (Scale3::new(1.0, 1.0, Scalar::NAN), 2),
            (Scale3::new(Scalar::INFINITY, 1.0, 1.0), 0),
        ] {
            assert_eq!(bad.unwrap_err().axis, axis);
        }
        assert!(
            Scale3::uniform(-2.0)
                .unwrap_err()
                .to_string()
                .contains("scale[0]")
        );
    }
}
