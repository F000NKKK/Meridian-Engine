//! Invalid inputs to checked rotation constructors.

/// Why a rotation could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationError {
    /// An input component or angle was NaN or infinite.
    NonFinite,
    /// A direction or axis had zero length.
    ZeroDirection,
    /// Forward and up are parallel within the look-rotation tolerance.
    ParallelUp,
    /// Raw coefficients contain non-rotation blades or have zero norm.
    InvalidRotor,
    /// Interpolation fraction is outside the closed interval [0, 1].
    InvalidFraction,
}

impl core::fmt::Display for RotationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::NonFinite => "rotation requires finite components and an angle in radians",
            Self::ZeroDirection => "rotation requires a nonzero axis or direction",
            Self::ParallelUp => "look rotation requires independent forward and up directions",
            Self::InvalidRotor => "rotation requires nonzero scalar/bivector coefficients only",
            Self::InvalidFraction => "interpolation fraction must be between zero and one",
        })
    }
}

impl core::error::Error for RotationError {}
