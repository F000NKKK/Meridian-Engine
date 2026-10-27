//! One vertex attribute within a layout.

use crate::VertexFormat;

/// One vertex attribute's shape within a [`VertexLayout`](crate::VertexLayout): which
/// `@location` it binds to in the shader, its component format, and its
/// byte offset within one vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexAttributeDesc {
    pub location: u32,
    pub format: VertexFormat,
    pub offset: u64,
}
