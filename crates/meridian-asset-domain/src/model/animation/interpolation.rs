//! How a channel blends between keys.

/// glTF's interpolation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelInterpolation {
    /// Blend linearly (rotations along the shorter arc). glTF's default.
    #[default]
    Linear,
    /// Hold each key's value until the next key.
    Step,
    /// Hermite spline: each key stores an in-tangent, a value and an
    /// out-tangent.
    CubicSpline,
}
