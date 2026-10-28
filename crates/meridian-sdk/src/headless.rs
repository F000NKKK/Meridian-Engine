//! A [`GraphicsSession`] with no window: frames come back as pixels.

use meridian_rendering_domain::Scene3d;
use meridian_rendering_wgpu::{Device, DeviceError, Surface};

use crate::GraphicsSession;

impl GraphicsSession {
    /// A session rendering `width` x `height` frames on a headless device,
    /// for tools that need no display (thumbnails, an editor viewport,
    /// benchmarks, captured video).
    ///
    /// # Errors
    ///
    /// When no GPU adapter or device can be opened.
    pub async fn headless(width: u32, height: u32) -> Result<Self, DeviceError> {
        let device = Device::new().await?;
        let surface = Surface::offscreen(&device, width, height);
        Ok(Self::new(device, surface, width, height))
    }

    /// Draws `scene` and returns the frame as tightly packed RGBA8 rows, top
    /// first. `None` unless the session was built by
    /// [`headless`](Self::headless), or when the frame could not be drawn.
    pub fn render_to_pixels(&mut self, scene: &Scene3d, clear_color: [f64; 4]) -> Option<Vec<u8>> {
        let size = self.render.recorder.frame_size();
        self.render.render(scene, clear_color, size);
        self.render.recorder.read_pixels()
    }
}
