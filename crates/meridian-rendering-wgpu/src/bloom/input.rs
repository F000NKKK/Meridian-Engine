//! What bloom reads from the frame that was just drawn.

use crate::Texture;
use crate::submission::{PreparedScene, SceneRenderer, TextureRegistry};

/// The already-drawn scene [`BloomPass::apply`](super::BloomPass::apply)
/// extracts emissive light from and composites onto.
#[derive(Debug, Clone, Copy)]
pub struct BloomInput<'a> {
    /// Supplies the uploaded meshes and texture sampler.
    pub renderer: &'a SceneRenderer,
    /// The batches and view uniform to redraw through emissive extraction.
    pub prepared: &'a PreparedScene,
    /// Resolves the emissive maps of the batches.
    pub textures: &'a TextureRegistry,
    /// The HDR scene target the main pass drew into, which bloom adds onto.
    pub target: &'a Texture,
}
