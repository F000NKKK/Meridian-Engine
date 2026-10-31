//! Decoded image pixels.

/// Decoded image data (RGBA8 pixels + dimensions), not yet uploaded to the GPU.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageAsset {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Row-major, top-to-bottom, 4 bytes (R,G,B,A) per pixel.
    pub pixels: Vec<u8>,
}
