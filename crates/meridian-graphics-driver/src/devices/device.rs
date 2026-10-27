//! The windowed graphics device: a GPU device plus surface-aware pipeline creation.

use meridian_platform_core::{BackendCapabilities, CpuCapabilities, GpuCapabilities};

use crate::{BufferUsage, CommandBuffer, DeviceError, Surface};

/// A windowed-capable GPU device. Wraps [`meridian_gpu_driver::Device`]
/// (which owns the actual `wgpu::Device`/`wgpu::Queue` and every
/// buffer/shader mechanic) and adds nothing but the graphics-specific
/// constructors/resources declared in this crate — buffer/shader/texture
/// methods forward directly to the inner device, not reimplemented here.
#[derive(Debug)]
pub struct Device(pub(crate) meridian_gpu_driver::Device);

impl Device {
    /// Requests a headless (no-surface) GPU device — see
    /// [`meridian_gpu_driver::Device::new`].
    pub async fn new() -> Result<Self, DeviceError> {
        Ok(Self(meridian_gpu_driver::Device::new().await?))
    }

    /// Requests a windowed GPU device plus its swapchain [`Surface`](crate::Surface),
    /// already configured at `width`x`height`. `target` is anything
    /// `wgpu` can build a surface from — in practice always
    /// `Arc<winit::window::Window>` via `platform_core::Window::surface_target`,
    /// but this crate never names `winit`'s type itself (see the module
    /// doc). Picks the first sRGB-capable format the surface reports
    /// (falling back to whatever format is first if none is sRGB) — sRGB
    /// output is the conventional default for correct-looking color
    /// without every shader hand-rolling its own gamma correction.
    pub async fn new_windowed(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<(Self, Surface), DeviceError> {
        let (device, surface, adapter) = meridian_gpu_driver::Device::new_windowed(target).await?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: width.max(1),
            height: height.max(1),
            present_mode: capabilities.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: Vec::new(),
        };
        surface.configure(device.wgpu_device(), &config);

        Ok((
            Self(device),
            Surface {
                raw: surface,
                config,
            },
        ))
    }

    /// The adapter's reported name — see
    /// [`meridian_gpu_driver::Device::adapter_name`].
    pub fn adapter_name(&self) -> &str {
        self.0.adapter_name()
    }

    pub fn create_buffer(
        &self,
        byte_len: usize,
        usage: BufferUsage,
    ) -> meridian_gpu_driver::Buffer {
        self.0.create_buffer(byte_len, usage)
    }

    pub fn write_buffer(&self, buffer: &meridian_gpu_driver::Buffer, data: &[u8]) {
        self.0.write_buffer(buffer, data);
    }

    pub async fn read_buffer(&self, buffer: &meridian_gpu_driver::Buffer) -> Vec<u8> {
        self.0.read_buffer(buffer).await
    }

    pub fn create_shader(&self, label: &str, wgsl_source: &str) -> meridian_gpu_driver::Shader {
        self.0.create_shader(label, wgsl_source)
    }

    /// Opens a new [`CommandBuffer`](crate::CommandBuffer) for recording. Nothing reaches the
    /// GPU until [`CommandBuffer::submit`](crate::CommandBuffer::submit) is called.
    pub fn create_command_buffer(&self) -> CommandBuffer<'_> {
        CommandBuffer {
            inner: self.0.create_command_buffer(),
        }
    }
}

impl BackendCapabilities for Device {
    fn cpu(&self) -> CpuCapabilities {
        self.0.cpu()
    }

    fn gpu(&self) -> Option<GpuCapabilities> {
        self.0.gpu()
    }
}
