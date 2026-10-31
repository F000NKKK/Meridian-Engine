//! One vertex attribute within a layout.

use crate::VertexFormat;

/// One vertex attribute's shape within a [`VertexLayout`](crate::VertexLayout): which
/// `@location` it binds to in the shader, its component format, and its
/// byte offset within one vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexAttributeDesc {
    /// Shader input location for this attribute.
    pub location: u32,
    /// Component count and scalar representation.
    pub format: VertexFormat,
    /// Byte offset from the start of one vertex.
    pub offset: u64,
}
