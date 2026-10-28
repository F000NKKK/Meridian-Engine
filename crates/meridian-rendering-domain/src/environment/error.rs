//! Why an environment could not be built.

/// Why [`EnvironmentMap::from_equirect`](super::EnvironmentMap::from_equirect)
/// refused an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentError {
    /// A dimension is zero.
    Empty,
    /// The texel count is not `width * height`.
    TexelCountMismatch {
        /// `width * height`.
        expected: usize,
        /// How many texels were given.
        actual: usize,
    },
}

impl core::fmt::Display for EnvironmentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => write!(f, "environment image has a zero dimension"),
            Self::TexelCountMismatch { expected, actual } => {
                write!(f, "environment image needs {expected} texels, got {actual}")
            }
        }
    }
}

impl std::error::Error for EnvironmentError {}

impl meridian_diagnostics::EngineError for EnvironmentError {}
