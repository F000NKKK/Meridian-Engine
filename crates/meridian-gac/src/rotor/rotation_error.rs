//! Invalid inputs to checked rotation constructors.

/// Why a rotation could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationError {
    /// An input component or angle was NaN or infinite.
    NonFinite,
    /// A direction or axis had zero length.
    ZeroDirection,
}

impl core::fmt::Display for RotationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::NonFinite => "rotation requires finite components and an angle in radians",
            Self::ZeroDirection => "rotation requires a nonzero axis or direction",
        })
    }
}

impl core::error::Error for RotationError {}
