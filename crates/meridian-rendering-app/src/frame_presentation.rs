//! Immutable presentation parameters for one frame.

/// Presentation configuration shared by scene and viewport render paths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePresentation {
    /// Background color as linear RGBA.
    pub clear_color: [f64; 4],
    /// Window target size in physical pixels.
    pub target_size: (u32, u32),
}
