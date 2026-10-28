//! How a track blends between two keyframes.

/// glTF's three interpolation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Holds a key's value until the next key.
    Step,
    /// Blends linearly; rotations take the shorter arc at constant speed.
    #[default]
    Linear,
    /// A cubic Hermite spline: every key carries an in-tangent, a value and an
    /// out-tangent.
    CubicSpline,
}

impl Interpolation {
    /// How many values a key carries: three for a spline, one otherwise.
    #[must_use]
    pub fn values_per_key(self) -> usize {
        match self {
            Self::CubicSpline => 3,
            Self::Step | Self::Linear => 1,
        }
    }
}
