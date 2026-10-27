//! Decoded image pixels.

/// Decoded image data (RGBA8 pixels + dimensions), not yet uploaded to the GPU.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageAsset {
    pub width: u32,
    pub height: u32,
    /// Row-major, top-to-bottom, 4 bytes (R,G,B,A) per pixel.
    pub pixels: Vec<u8>,
}
