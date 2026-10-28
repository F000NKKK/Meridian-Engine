//! Draws overlays over a finished 3D frame.

use meridian_rendering_domain::{Overlay2d, SpriteView, batch_scene2d_clipped};

use crate::{
    CommandBuffer, Device, PreparedSprites, SpriteRenderer, Surface, SurfaceFrame, TextureRegistry,
};

/// The sprite renderer an overlay pass draws with. Overlays go over the
/// bloomed 3D result: no depth, no blur, slice order.
#[derive(Debug)]
pub(crate) struct OverlayPass {
    sprites: SpriteRenderer,
}

impl OverlayPass {
    pub(crate) fn new(device: &Device, surface: &Surface) -> Self {
        Self {
            sprites: SpriteRenderer::new(device, surface),
        }
    }

    /// Batches and uploads each overlay; must run before any pass opens.
    pub(crate) fn prepare(
        &self,
        device: &Device,
        textures: &TextureRegistry,
        overlays: &[Overlay2d],
        viewport: [u32; 2],
    ) -> Vec<PreparedSprites> {
        overlays
            .iter()
            .map(|overlay| {
                let batches = batch_scene2d_clipped(&overlay.scene, &overlay.clips);
                let view = SpriteView {
                    camera: overlay.scene.camera,
                    viewport,
                };
                self.sprites.prepare(device, view, &batches, textures)
            })
            .collect()
    }

    /// Records every prepared overlay into one pass that keeps the 3D frame
    /// underneath.
    pub(crate) fn draw(
        &self,
        commands: &mut CommandBuffer<'_>,
        frame: &SurfaceFrame,
        prepared: &[PreparedSprites],
    ) {
        if prepared.is_empty() {
            return;
        }
        let mut pass = commands.begin_render_pass_loaded(frame.view());
        for sprites in prepared {
            self.sprites.draw(&mut pass, sprites);
        }
    }
}
