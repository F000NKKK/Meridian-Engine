//! The result of an extraction that keeps clipping.

use meridian_rendering_domain::{ClipSpan, Scene2d};

/// A UI as sprites plus the clip rectangles of its scroll viewers. Draw it
/// with `batch_scene2d_clipped(&extracted.scene, &extracted.clips)`.
#[derive(Debug)]
pub struct ExtractedScene {
    /// The sprites, in painter's order.
    pub scene: Scene2d,
    /// Where each scroll viewer's content shows; nested viewers nest.
    pub clips: Vec<ClipSpan>,
}
