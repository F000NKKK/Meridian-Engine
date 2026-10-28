//! Decoded high-dynamic-range image pixels.

/// Decoded HDR image data (linear RGB, `f32` per channel), not yet uploaded
/// to the GPU. Radiance values are unbounded above 1.0.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HdrImageAsset {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major, top-to-bottom, one linear `[r, g, b]` triple per pixel.
    pub pixels: Vec<[f32; 3]>,
}
