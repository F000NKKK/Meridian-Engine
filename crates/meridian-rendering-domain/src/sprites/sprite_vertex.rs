//! One vertex of a sprite triangle list.

/// A vertex in camera units. `color` is a straight (not premultiplied)
/// RGBA tint multiplied with the sampled texel; the shader premultiplies
/// the product before blending.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteVertex {
    /// Position in the camera's units.
    pub position: [f32; 2],
    /// Normalized texture coordinates, `(0, 0)` at the top-left texel.
    pub uv: [f32; 2],
    /// Straight RGBA tint.
    pub color: [f32; 4],
}
