//! A frame's sprite batches, uploaded.

use core::ops::Range;

use meridian_graphics_driver::{BindGroup, Buffer};

/// One draw: a texture's bind group and its slice of the vertex buffer.
#[derive(Debug)]
pub(crate) struct SpriteRun {
    pub bind_group: BindGroup,
    pub vertices: Range<u32>,
}

/// Built by [`SpriteRenderer::prepare`](super::SpriteRenderer::prepare)
/// before the render pass opens and kept alive until the pass is done.
/// Owns its own camera uniform, so several prepared scenes can coexist in
/// one frame.
#[derive(Debug, Default)]
pub struct PreparedSprites {
    pub(crate) vertices: Option<Buffer>,
    pub(crate) runs: Vec<SpriteRun>,
}

impl PreparedSprites {
    /// Number of draw calls recorded by [`SpriteRenderer::draw`](super::SpriteRenderer::draw).
    #[must_use]
    pub fn draw_calls(&self) -> usize {
        self.runs.len()
    }
}
