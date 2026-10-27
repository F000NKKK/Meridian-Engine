//! One draw call of a prepared sprite scene.

use core::ops::Range;

use meridian_graphics_driver::BindGroup;

use super::ScissorRect;

/// One draw: a texture's bind group and its slice of the vertex buffer.
#[derive(Debug)]
pub(crate) struct SpriteRun {
    pub bind_group: BindGroup,
    pub vertices: Range<u32>,
    pub scissor: Option<ScissorRect>,
}
