//! The axis one blur pass smears along.

/// Which axis a blur pass reads along; the shader receives it as a
/// `vec2<f32>` direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlurDirection {
    /// Along x: direction `(1, 0)`.
    Horizontal,
    /// Along y: direction `(0, 1)`.
    Vertical,
}

impl BlurDirection {
    /// The `(x, y)` direction the shader multiplies the texel size by.
    pub(crate) fn components(self) -> [f32; 2] {
        match self {
            Self::Horizontal => [1.0, 0.0],
            Self::Vertical => [0.0, 1.0],
        }
    }
}
