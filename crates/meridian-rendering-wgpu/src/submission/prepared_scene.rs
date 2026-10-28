//! A scene baked for drawing.

use meridian_rendering_domain::ShadowLight;

use super::PreparedBatch;
use crate::Buffer;

/// What [`SceneRenderer::render_shadow_pass`](super::SceneRenderer::render_shadow_pass)
/// hands to the main pass: the batches in drawing order, the instance
/// buffer they index (both kept alive until the pass is done with them) and
/// the light that cast the shadows.
#[derive(Debug)]
pub struct PreparedScene {
    /// One entry per instanced draw: opaque first, then translucent far to
    /// near, then on-top draws.
    pub batches: Vec<PreparedBatch>,
    /// One record per drawn renderable, `None` when nothing is visible.
    pub(crate) instances: Option<Buffer>,
    /// The shadow-casting light of this view.
    pub shadow: ShadowLight,
}

impl PreparedScene {
    /// How many renderables the scene draws, across all batches: the visible
    /// ones (ADR 046).
    #[must_use]
    pub fn instance_count(&self) -> u32 {
        self.batches.iter().map(PreparedBatch::instance_count).sum()
    }
}
