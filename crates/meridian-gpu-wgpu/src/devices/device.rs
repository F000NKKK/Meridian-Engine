//! The GPU device: adapter, queue and resource creation entry points.

use meridian_platform_winit::{BackendCapabilities, CpuCapabilities, GpuCapabilities};

use crate::{CommandBuffer, DeviceError};

/// A GPU device: owns the `wgpu::Device`/`wgpu::Queue` pair every other
/// type here is created through or submitted against. `Clone` is cheap
/// (`wgpu::Device`/`Queue` are `Arc`-backed internally) — useful for
/// callers that want to hand the same logical device to more than one
/// consumer (e.g. `compute-app::ComputeContext` holding one
/// alongside its CPU backend).
#[derive(Debug, Clone)]
pub struct Device {
    pub(crate) raw: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
}

impl Device {
    /// Requests a headless (no-surface) GPU device: an `Instance` with
    /// every backend `wgpu` supports on this platform enabled, the
    /// adapter it picks (`compatible_surface: None`), then the logical
    /// device off that adapter. A real `async fn` — see the module doc.
    ///
    /// # Errors
    ///
    /// [`DeviceError::NoAdapter`] when no adapter is available, or
    /// [`DeviceError::RequestDevice`] when the adapter cannot open a device.
    pub async fn new() -> Result<Self, DeviceError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let (device, queue, adapter_info, _adapter) =
            Self::request_from_instance(&instance, None).await?;
        Ok(Self {
            raw: device,
            queue,
            adapter_info,
        })
    }

    /// Requests a device compatible with presenting to `target`, and
    /// hands back the raw `wgpu::Surface` plus the `wgpu::Adapter` it was
    /// chosen against alongside it. The adapter is returned (unlike
    /// [`Device::new`], which discards it) because `wgpu` ties a
    /// surface's format/present-mode capabilities to the adapter, not the
    /// device — `rendering-wgpu`'s own surface configuration needs it
    /// for `Surface::get_capabilities`, and this is the only place that
    /// adapter is ever available; see the module doc for why surface
    /// *configuration*/lifecycle isn't otherwise this crate's job.
    ///
    /// # Errors
    ///
    /// [`DeviceError::CreateSurface`] when `target` cannot become a surface,
    /// plus the errors of [`Device::new`].
    pub async fn new_windowed(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
    ) -> Result<(Self, wgpu::Surface<'static>, wgpu::Adapter), DeviceError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance
            .create_surface(target)
            .map_err(DeviceError::CreateSurface)?;
        let (device, queue, adapter_info, adapter) =
            Self::request_from_instance(&instance, Some(&surface)).await?;
        Ok((
            Self {
                raw: device,
                queue,
                adapter_info,
            },
            surface,
            adapter,
        ))
    }

    async fn request_from_instance(
        instance: &wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<(wgpu::Device, wgpu::Queue, wgpu::AdapterInfo, wgpu::Adapter), DeviceError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface,
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await
            .map_err(|_| DeviceError::NoAdapter)?;
        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("meridian-gpu-wgpu device"),
                required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await
            .map_err(DeviceError::RequestDevice)?;
        meridian_diagnostics::log_info!(
            "GPU device acquired: {} ({:?}, {:?} backend)",
            adapter_info.name,
            adapter_info.device_type,
            adapter_info.backend
        );
        Ok((device, queue, adapter_info, adapter))
    }

    /// The adapter's reported name (e.g. `"NVIDIA GeForce RTX 4050
    /// Max-Q"`) — for logging/diagnostics, not parsed for behavior.
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        &self.adapter_info.name
    }

    /// Whether the device was opened with timestamp queries (GPU pass timing),
    /// which is requested whenever the adapter offers them.
    #[must_use]
    pub fn supports_timestamps(&self) -> bool {
        self.raw
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
    }

    /// The raw `wgpu::Device` — an escape hatch for `rendering-wgpu`'s
    /// own additional operations this crate doesn't know about (surface
    /// configuration, render pipeline creation). Prefer this crate's own
    /// methods (`create_buffer`, `create_shader`, ...) where they cover
    /// the need; reach for this only when they don't.
    #[must_use]
    pub fn wgpu_device(&self) -> &wgpu::Device {
        &self.raw
    }

    /// The raw `wgpu::Queue` — see [`Device::wgpu_device`]'s doc comment
    /// for why this is exposed (`rendering-wgpu` needs it for
    /// `Queue::present`, which isn't a concept this crate knows about).
    #[must_use]
    pub fn wgpu_queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Opens a new [`CommandBuffer`](crate::CommandBuffer) for recording. Nothing reaches the
    /// GPU until [`CommandBuffer::submit`](crate::CommandBuffer::submit) is called.
    #[must_use]
    pub fn create_command_buffer(&self) -> CommandBuffer<'_> {
        let encoder = self
            .raw
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        CommandBuffer {
            device: self,
            encoder,
        }
    }
}

impl BackendCapabilities for Device {
    /// Neither `rendering-wgpu` nor `compute-wgpu`'s GPU path has a
    /// CPU-dispatch mechanism of its own — this reports the machine's CPU
    /// thread count for consistency with every other `*-wgpu`/`*-cpal` adapter's
    /// `BackendCapabilities` impl (`compute-wgpu`'s own CPU backend
    /// reports this too, independently of this GPU device).
    fn cpu(&self) -> CpuCapabilities {
        CpuCapabilities::detect()
    }

    /// Always `Some` — a `Device` only exists once [`Device::new`]/
    /// [`Device::new_windowed`] has already found a real adapter.
    fn gpu(&self) -> Option<GpuCapabilities> {
        Some(GpuCapabilities {
            device_name: self.adapter_info.name.clone(),
        })
    }
}
