//! A 2D scene drawn over the 3D world.

use meridian_graphics_core::{ClipSpan, Scene2d};

/// A [`Scene2d`] (UI, HUD, debug text) drawn over the 3D scene, after
/// bloom, with its clip spans. Overlays draw in slice order, so a later
/// one covers an earlier one.
#[derive(Debug, Clone, Default)]
pub struct Overlay2d {
    /// What to draw.
    pub scene: Scene2d,
    /// Clip rectangles by layer range; empty for no clipping.
    pub clips: Vec<ClipSpan>,
}

impl Overlay2d {
    /// An unclipped overlay.
    #[must_use]
    pub fn new(scene: Scene2d) -> Self {
        Self {
            scene,
            clips: Vec::new(),
        }
    }
}
