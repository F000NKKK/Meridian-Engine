//! A scene baked for drawing.

use meridian_rendering_domain::ShadowLight;

use super::DrawBuffers;

/// What [`SceneRenderer::render_shadow_pass`](super::SceneRenderer::render_shadow_pass)
/// hands to the main pass: the baked draw buffers (kept alive until the
/// pass is done with them) and the light that cast the shadows.
#[derive(Debug)]
pub struct PreparedScene {
    /// One baked entry per drawable renderable.
    pub buffers: Vec<DrawBuffers>,
    /// Indices into `buffers` in drawing order: opaque, translucent far to
    /// near, then on-top draws.
    pub order: Vec<usize>,
    /// The shadow-casting light of this view.
    pub shadow: ShadowLight,
}
