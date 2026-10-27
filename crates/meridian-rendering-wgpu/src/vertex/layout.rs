//! A vertex buffer layout.

use crate::VertexAttributeDesc;

/// Describes one vertex buffer's layout for
/// [`Device::create_render_pipeline`](crate::Device::create_render_pipeline): `stride` is the byte size of one
/// vertex, `attributes` is where each field lives within it.
#[derive(Debug, Clone)]
pub struct VertexLayout {
    pub stride: u64,
    pub attributes: Vec<VertexAttributeDesc>,
}
