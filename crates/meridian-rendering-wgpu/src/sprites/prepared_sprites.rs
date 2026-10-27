//! A frame's sprite batches, uploaded.

use super::sprite_run::SpriteRun;
use crate::Buffer;

/// Built by [`SpriteRenderer::prepare`](super::SpriteRenderer::prepare)
/// before the render pass opens and kept alive until the pass is done.
/// Owns its own camera uniform, so several prepared scenes can coexist in
/// one frame.
#[derive(Debug, Default)]
pub struct PreparedSprites {
    pub(crate) vertices: Option<Buffer>,
    pub(crate) runs: Vec<SpriteRun>,
    pub(crate) viewport: [u32; 2],
}

impl PreparedSprites {
    /// Number of draw calls recorded by [`SpriteRenderer::draw`](super::SpriteRenderer::draw).
    #[must_use]
    pub fn draw_calls(&self) -> usize {
        self.runs.len()
    }
}
