//! One draw call of a prepared sprite scene.

use core::ops::Range;

use meridian_rendering_domain::ScissorRect;

/// One draw: the index of a texture's bind group in
/// [`PreparedSprites`](super::PreparedSprites) and its slice of the vertex buffer.
#[derive(Debug)]
pub(crate) struct SpriteRun {
    pub group: usize,
    pub vertices: Range<u32>,
    pub scissor: Option<ScissorRect>,
}
