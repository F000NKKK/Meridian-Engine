//! What bloom reads from the frame that was just drawn.

use crate::SurfaceFrame;
use crate::submission::{DrawBuffers, SceneRenderer};

/// The already-drawn frame [`BloomPass::apply`](super::BloomPass::apply)
/// extracts emissive light from and composites onto.
#[derive(Debug, Clone, Copy)]
pub struct BloomInput<'a> {
    /// Supplies the shared view/lighting uniform buffer.
    pub renderer: &'a SceneRenderer,
    /// The draws to redraw through the emissive-extraction pipelines.
    pub draw_buffers: &'a [DrawBuffers],
    /// The frame the main pass drew into, which bloom adds onto.
    pub target_frame: &'a SurfaceFrame,
}
