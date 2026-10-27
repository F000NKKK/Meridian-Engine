//! A window surface frames are acquired from.

use crate::{AcquireFrameError, Device, SurfaceFrame};

/// A swapchain: the sequence of textures a windowed [`Device`](crate::Device) presents
/// to the screen. Built by [`Device::new_windowed`](crate::Device::new_windowed); call
/// [`Surface::resize`] whenever the window's size changes (skipping a
/// stale-sized frame is worse than briefly rendering at the old size, but
/// never configuring a `0`x`0` surface — [`Surface::resize`] guards that),
/// and [`Surface::acquire_frame`] once per frame to get something to
/// render into.
#[derive(Debug)]
pub struct Surface {
    pub(crate) raw: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
}

impl Surface {
    /// The color format frames from this surface are in — what
    /// [`Device::create_render_pipeline`](crate::Device::create_render_pipeline)'s color target must match.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Reconfigures the surface for a new size. A no-op if either
    /// dimension is `0` (a minimized window reports this on some
    /// platforms) — configuring a zero-sized surface is a `wgpu` panic,
    /// not a recoverable error, so this guards it here rather than
    /// pushing that footgun onto every caller.
    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.raw.configure(device.0.wgpu_device(), &self.config);
    }

    /// Acquires the next frame to render into. `Err` on a transient
    /// swapchain problem (e.g. the surface was resized by the OS but not
    /// yet reconfigured via [`Surface::resize`], or the window is
    /// currently occluded/minimized) — a caller should treat that as
    /// "skip this frame," not a fatal error.
    pub fn acquire_frame(&self) -> Result<SurfaceFrame, AcquireFrameError> {
        match self.raw.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => {
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                Ok(SurfaceFrame { output, view })
            }
            wgpu::CurrentSurfaceTexture::Timeout => Err(AcquireFrameError::Timeout),
            wgpu::CurrentSurfaceTexture::Occluded => Err(AcquireFrameError::Occluded),
            wgpu::CurrentSurfaceTexture::Outdated => Err(AcquireFrameError::Outdated),
            wgpu::CurrentSurfaceTexture::Lost => Err(AcquireFrameError::Lost),
            wgpu::CurrentSurfaceTexture::Validation => Err(AcquireFrameError::Validation),
        }
    }
}
