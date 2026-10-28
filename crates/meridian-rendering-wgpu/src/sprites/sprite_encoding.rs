//! How a `SpriteVertex` is laid out in a vertex buffer.

use meridian_rendering_domain::SpriteVertex;

use crate::{VertexAttributeDesc, VertexFormat, VertexLayout, VertexStep};

/// Bytes in one encoded sprite vertex: 2 + 2 + 4 floats.
pub const SPRITE_VERTEX_BYTES: usize = 32;

/// The vertex layout the sprite pipeline reads.
pub(crate) fn sprite_vertex_layout() -> VertexLayout {
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
        step: VertexStep::Vertex,
    }
}

/// Appends `vertex` as 32 little-endian bytes.
pub(crate) fn write_vertex(vertex: &SpriteVertex, bytes: &mut Vec<u8>) {
    let floats = vertex
        .position
        .iter()
        .chain(&vertex.uv)
        .chain(&vertex.color);
    bytes.extend(floats.flat_map(|value| value.to_le_bytes()));
}
