//! One acquired surface frame.

use crate::Device;

/// One acquired swapchain frame: render into [`SurfaceFrame::view`], then
/// [`SurfaceFrame::present`] to hand it back to the OS compositor.
#[derive(Debug)]
pub struct SurfaceFrame {
    pub(crate) output: wgpu::SurfaceTexture,
    pub(crate) view: wgpu::TextureView,
}

impl SurfaceFrame {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Presents this frame via `device`'s queue.
    pub fn present(self, device: &Device) {
        device.0.wgpu_queue().clone().present(self.output);
    }
}
