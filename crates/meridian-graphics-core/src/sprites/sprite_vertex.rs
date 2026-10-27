//! One vertex of a sprite triangle list.

use meridian_graphics_driver::{VertexAttributeDesc, VertexFormat, VertexLayout};

/// Bytes in one encoded [`SpriteVertex`]: 2 + 2 + 4 floats.
pub const SPRITE_VERTEX_BYTES: usize = 32;

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

impl SpriteVertex {
    pub(crate) fn layout() -> VertexLayout {
        let attribute = |location, format, offset| VertexAttributeDesc {
            location,
            format,
            offset,
        };
        VertexLayout {
            stride: SPRITE_VERTEX_BYTES as u64,
            attributes: vec![
                attribute(0, VertexFormat::Float32x2, 0),
                attribute(1, VertexFormat::Float32x2, 8),
                attribute(2, VertexFormat::Float32x4, 16),
            ],
        }
    }

    /// Appends the vertex as 32 little-endian bytes.
    pub(crate) fn write_to(&self, bytes: &mut Vec<u8>) {
        let floats = self.position.iter().chain(&self.uv).chain(&self.color);
        bytes.extend(floats.flat_map(|value| value.to_le_bytes()));
    }
}
