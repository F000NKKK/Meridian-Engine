//! One acquired surface frame.

use crate::Device;

/// One acquired swapchain frame: render into [`SurfaceFrame::view`], then
/// [`SurfaceFrame::present`] to hand it back to the OS compositor.
#[derive(Debug)]
pub struct SurfaceFrame {
    pub(crate) output: Option<wgpu::SurfaceTexture>,
    pub(crate) view: wgpu::TextureView,
}

impl SurfaceFrame {
    /// Returns the texture view used as this frame's render target.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Presents this frame via `device`'s queue.
    pub fn present(self, device: &Device) {
        if let Some(output) = self.output {
            device.0.wgpu_queue().clone().present(output);
        }
    }
}
