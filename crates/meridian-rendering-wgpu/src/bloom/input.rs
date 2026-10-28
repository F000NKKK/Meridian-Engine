//! What bloom reads from the frame that was just drawn.

use crate::SurfaceFrame;
use crate::submission::{PreparedScene, SceneRenderer};

/// The already-drawn frame [`BloomPass::apply`](super::BloomPass::apply)
/// extracts emissive light from and composites onto.
#[derive(Debug, Clone, Copy)]
pub struct BloomInput<'a> {
    /// Supplies the shared view/lighting uniform buffer.
    pub renderer: &'a SceneRenderer,
    /// The batches to redraw through the emissive-extraction pipeline.
    pub prepared: &'a PreparedScene,
    /// The frame the main pass drew into, which bloom adds onto.
    pub target_frame: &'a SurfaceFrame,
}
