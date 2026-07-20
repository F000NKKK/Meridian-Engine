//! Shared low-level `wgpu` device/buffer/shader/compute-pipeline mechanics underneath `graphics-driver` and `compute-driver`. Knows nothing about scenes, materials, render passes/swapchains, or compute-dispatch scheduling policy — see docs/adr/011-shared-gpu-driver-crate.md.
//!
//! Both `graphics-driver` (rendering) and `compute-driver` (general GPU
//! compute) need the same underlying primitives: acquire an adapter/
//! device, allocate/read/write buffers, compile a shader, build a compute
//! pipeline and dispatch it. Duplicating that wgpu-wrapping code in both
//! crates (which is what an earlier pass of this workspace did) is
//! exactly the kind of cross-crate duplication CLAUDE.md's "don't drag
//! another crate's logic into your own" rule exists to catch — so it
//! lives here once instead, the same shape `meridian-gac-compute` already
//! established for "two crates independently need the same underlying
//! thing" (see [ADR 007](../../../docs/adr/007-batch-transforms-via-compute.md)),
//! just at the driver layer instead of the domain layer.
//!
//! **Windowed surface creation is split deliberately.** [`Device::new_windowed`]
//! only does the adapter/device *handshake* with `compatible_surface`
//! set, and hands back the raw `wgpu::Surface` — it does not own
//! `SurfaceConfiguration`/swapchain lifecycle/present, which is a
//! graphics-specific concern `graphics-driver` builds on top (its own
//! `Surface` type). `compute-driver` never touches this constructor at
//! all — it only ever needs [`Device::new`] (headless).
//!
//! **Async on genuine I/O, not on everything** — same policy as every
//! other driver crate in this workspace (see
//! [ADR 009](../../../docs/adr/009-async-io-via-tokio.md)): [`Device::new`]/
//! [`Device::new_windowed`] (an OS/driver handshake) and
//! [`Device::read_buffer`] (waiting on in-flight GPU work) are real
//! `async fn`s; allocation/recording calls stay synchronous.

use std::sync::mpsc;

use meridian_platform_core::{BackendCapabilities, CpuCapabilities, GpuCapabilities};

/// A GPU device: owns the `wgpu::Device`/`wgpu::Queue` pair every other
/// type here is created through or submitted against. `Clone` is cheap
/// (`wgpu::Device`/`Queue` are `Arc`-backed internally) — useful for
/// callers that want to hand the same logical device to more than one
/// consumer (e.g. `compute-runtime::ComputeContext` holding one
/// alongside its CPU backend).
#[derive(Debug, Clone)]
pub struct Device {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
}

/// Why [`Device::new`]/[`Device::new_windowed`] failed. Every variant is a
/// real, reachable failure — not a "shouldn't happen" case — since
/// adapter/device acquisition depends on what's actually installed on the
/// machine running this code (driver, ICD, `wgpu` backend availability).
#[derive(Debug)]
pub enum DeviceError {
    /// No adapter matched the request (no GPU, no compatible driver, or
    /// `wgpu` has no backend for this platform).
    NoAdapter,
    RequestDevice(wgpu::RequestDeviceError),
    /// [`Device::new_windowed`] only: the window/display handle couldn't
    /// be turned into a `wgpu::Surface` (e.g. an unsupported platform
    /// backend).
    CreateSurface(wgpu::CreateSurfaceError),
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceError::NoAdapter => write!(f, "no compatible GPU adapter found"),
            DeviceError::RequestDevice(e) => write!(f, "failed to request GPU device: {e}"),
            DeviceError::CreateSurface(e) => write!(f, "failed to create GPU surface: {e}"),
        }
    }
}

impl std::error::Error for DeviceError {}

impl Device {
    /// Requests a headless (no-surface) GPU device: an `Instance` with
    /// every backend `wgpu` supports on this platform enabled, the
    /// adapter it picks (`compatible_surface: None`), then the logical
    /// device off that adapter. A real `async fn` — see the module doc.
    pub async fn new() -> Result<Self, DeviceError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let (device, queue, adapter_info, _adapter) =
            Self::request_from_instance(&instance, None).await?;
        Ok(Self {
            device,
            queue,
            adapter_info,
        })
    }

    /// Requests a device compatible with presenting to `target`, and
    /// hands back the raw `wgpu::Surface` plus the `wgpu::Adapter` it was
    /// chosen against alongside it. The adapter is returned (unlike
    /// [`Device::new`], which discards it) because `wgpu` ties a
    /// surface's format/present-mode capabilities to the adapter, not the
    /// device — `graphics-driver`'s own surface configuration needs it
    /// for `Surface::get_capabilities`, and this is the only place that
    /// adapter is ever available; see the module doc for why surface
    /// *configuration*/lifecycle isn't otherwise this crate's job.
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
                device,
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
                label: Some("meridian-gpu-driver device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await
            .map_err(DeviceError::RequestDevice)?;
        Ok((device, queue, adapter_info, adapter))
    }

    /// The adapter's reported name (e.g. `"NVIDIA GeForce RTX 4050
    /// Max-Q"`) — for logging/diagnostics, not parsed for behavior.
    pub fn adapter_name(&self) -> &str {
        &self.adapter_info.name
    }

    /// The raw `wgpu::Device` — an escape hatch for `graphics-driver`'s
    /// own additional operations this crate doesn't know about (surface
    /// configuration, render pipeline creation). Prefer this crate's own
    /// methods (`create_buffer`, `create_shader`, ...) where they cover
    /// the need; reach for this only when they don't.
    pub fn wgpu_device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The raw `wgpu::Queue` — see [`Device::wgpu_device`]'s doc comment
    /// for why this is exposed (`graphics-driver` needs it for
    /// `Queue::present`, which isn't a concept this crate knows about).
    pub fn wgpu_queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Allocates a GPU-visible buffer of `byte_len` bytes. Always usable
    /// as a copy source/destination in addition to `usage`'s role — a
    /// generous default rather than fine-grained per-buffer usage-flag
    /// control, which is future work once a concrete need for tighter
    /// flags shows up.
    pub fn create_buffer(&self, byte_len: usize, usage: BufferUsage) -> Buffer {
        let raw = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: byte_len as u64,
            usage: usage.to_wgpu() | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Buffer { raw, byte_len }
    }

    /// Uploads `data` to `buffer` starting at byte offset 0.
    pub fn write_buffer(&self, buffer: &Buffer, data: &[u8]) {
        self.queue.write_buffer(&buffer.raw, 0, data);
    }

    /// Reads `buffer`'s entire contents back to the CPU. A real `async
    /// fn` — see the module doc. Copies into a `MAP_READ`-capable staging
    /// buffer first (most `buffer` usages, e.g. `STORAGE`, aren't
    /// directly mappable on discrete GPUs) — the standard `wgpu` readback
    /// pattern. `wgpu` has no reactor of its own to drive `map_async`'s
    /// completion callback — something has to call `wgpu::Device::poll`,
    /// which itself blocks the calling thread until the GPU catches up —
    /// so that poll runs inside [`tokio::task::spawn_blocking`] rather
    /// than on this `async fn`'s own task.
    pub async fn read_buffer(&self, buffer: &Buffer) -> Vec<u8> {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("meridian-gpu-driver readback staging buffer"),
            size: buffer.byte_len as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(&buffer.raw, 0, &staging, 0, buffer.byte_len as u64);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        let device = self.device.clone();
        tokio::task::spawn_blocking(move || device.poll(wgpu::PollType::wait_indefinitely()))
            .await
            .expect("device poll task panicked")
            .expect("device poll failed");
        rx.recv()
            .expect("map_async callback dropped without a response")
            .expect("failed to map staging buffer for readback");

        let data = slice
            .get_mapped_range()
            .expect("staging buffer was mapped but its range isn't readable")
            .to_vec();
        staging.unmap();
        data
    }

    /// A 2D texture in `format`, with `usage` flags the caller picks
    /// (depth/stencil formats support a narrower usage set than color
    /// formats on some backends, so this doesn't guess). `graphics-driver`'s
    /// `DepthTexture` is a thin wrapper calling this with `Depth32Float` +
    /// `RENDER_ATTACHMENT`; a color texture at `Rgba8Unorm` +
    /// `TEXTURE_BINDING | COPY_DST | COPY_SRC` is the other common case.
    pub fn create_texture(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Texture {
        let raw = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = raw.create_view(&wgpu::TextureViewDescriptor::default());
        Texture {
            raw,
            view,
            width,
            height,
        }
    }

    /// Compiles a WGSL shader module. `label` is for GPU-debugger/error
    /// message purposes only.
    pub fn create_shader(&self, label: &str, wgsl_source: &str) -> Shader {
        let raw = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
            });
        Shader { raw }
    }

    /// Builds a compute pipeline from a single entry point in `shader`.
    /// The bind group layout is auto-derived from the shader's own
    /// `@group`/`@binding` declarations (`layout: None` in `wgpu` terms)
    /// rather than hand-specified — correct and simple for the single
    /// binding-per-dispatch shape [`CommandBuffer::dispatch_compute`]
    /// supports today; an explicit layout becomes worth building once a
    /// pipeline needs more than one bound resource.
    pub fn create_compute_pipeline(&self, shader: &Shader, entry_point: &str) -> ComputePipeline {
        let raw = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: None,
                module: &shader.raw,
                entry_point: Some(entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        ComputePipeline { raw }
    }

    /// Builds a bind group binding `buffer` at `@group(0) @binding(0)` of
    /// `pipeline`'s auto-derived layout (see
    /// [`Device::create_compute_pipeline`]'s doc comment). Shared by the
    /// compute-dispatch path here and by `graphics-driver`'s uniform bind
    /// group for its render pipelines — same single-binding shape either
    /// way.
    pub fn create_single_buffer_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffer: &Buffer,
    ) -> BindGroup {
        self.create_bind_group(pipeline_layout, &[buffer])
    }

    /// Builds a bind group binding `buffers[i]` at `@group(0)
    /// @binding(i)` of `pipeline_layout`, for pipelines that need more
    /// than one bound resource (e.g. a compute kernel reading from one
    /// storage buffer and writing to another) —
    /// [`Device::create_single_buffer_bind_group`]'s general form.
    pub fn create_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffers: &[&Buffer],
    ) -> BindGroup {
        let entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, buffer)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buffer.raw.as_entire_binding(),
            })
            .collect();
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline_layout,
            entries: &entries,
        });
        BindGroup { raw }
    }

    /// Opens a new [`CommandBuffer`] for recording. Nothing reaches the
    /// GPU until [`CommandBuffer::submit`] is called.
    pub fn create_command_buffer(&self) -> CommandBuffer<'_> {
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        CommandBuffer {
            device: self,
            encoder,
        }
    }
}

impl BackendCapabilities for Device {
    /// Neither `graphics-driver` nor `compute-driver`'s GPU path has a
    /// CPU-dispatch mechanism of its own — this reports the machine's CPU
    /// thread count for consistency with every other `*-driver`'s
    /// `BackendCapabilities` impl (`compute-driver`'s own CPU backend
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

/// How a [`Buffer`] is meant to be bound. Maps to `wgpu::BufferUsages`;
/// every buffer also always gets `COPY_SRC | COPY_DST` — see
/// [`Device::create_buffer`]'s doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    Vertex,
    Index,
    Uniform,
    Storage,
}

impl BufferUsage {
    fn to_wgpu(self) -> wgpu::BufferUsages {
        match self {
            BufferUsage::Vertex => wgpu::BufferUsages::VERTEX,
            BufferUsage::Index => wgpu::BufferUsages::INDEX,
            BufferUsage::Uniform => wgpu::BufferUsages::UNIFORM,
            BufferUsage::Storage => wgpu::BufferUsages::STORAGE,
        }
    }
}

/// A recorded, submittable sequence of GPU commands. Borrows the
/// [`Device`] it was opened from ([`Device::create_command_buffer`]) so a
/// `CommandBuffer` can't outlive its device; nothing is submitted to the
/// GPU until [`CommandBuffer::submit`] consumes it.
#[derive(Debug)]
pub struct CommandBuffer<'a> {
    device: &'a Device,
    encoder: wgpu::CommandEncoder,
}

impl CommandBuffer<'_> {
    /// Records a compute dispatch: binds `buffer` at `@group(0)
    /// @binding(0)` (the only binding shape supported so far — see
    /// [`Device::create_compute_pipeline`]'s doc comment) and dispatches
    /// `workgroups` workgroups along X (Y/Z left at 1 — 1D dispatch is
    /// all today's callers need; extending to 3D is additive).
    pub fn dispatch_compute(
        &mut self,
        pipeline: &ComputePipeline,
        buffer: &Buffer,
        workgroups: u32,
    ) {
        let bind_group_layout = pipeline.raw.get_bind_group_layout(0);
        let bind_group = self
            .device
            .create_single_buffer_bind_group(&bind_group_layout, buffer);

        let mut pass = self
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline.raw);
        pass.set_bind_group(0, &bind_group.raw, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Escape hatch for `graphics-driver`'s render-pass recording, which
    /// this crate deliberately doesn't know about (render
    /// passes/swapchains are a graphics-specific concept — see the module
    /// doc). Prefer [`CommandBuffer::dispatch_compute`] where it covers
    /// the need.
    pub fn encoder_mut(&mut self) -> &mut wgpu::CommandEncoder {
        &mut self.encoder
    }

    /// The [`Device`] this buffer was opened from — needed by
    /// `graphics-driver`'s render-pass recording alongside
    /// [`CommandBuffer::encoder_mut`].
    pub fn device(&self) -> &Device {
        self.device
    }

    /// Submits every recorded command to the device's queue.
    pub fn submit(self) {
        self.device.queue.submit(Some(self.encoder.finish()));
    }
}

/// A GPU-visible buffer (vertex/index/uniform/storage).
#[derive(Debug)]
pub struct Buffer {
    raw: wgpu::Buffer,
    pub byte_len: usize,
}

impl Buffer {
    /// The raw `wgpu::Buffer` — an escape hatch for `graphics-driver`'s
    /// own additional buffer operations (setting it as a vertex/index
    /// buffer on a render pass) this crate doesn't know about.
    pub fn wgpu_buffer(&self) -> &wgpu::Buffer {
        &self.raw
    }
}

/// A GPU texture resource, with its default view already created.
#[derive(Debug)]
pub struct Texture {
    #[allow(dead_code)]
    raw: wgpu::Texture,
    view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

/// A compiled shader module.
#[derive(Debug)]
pub struct Shader {
    raw: wgpu::ShaderModule,
}

impl Shader {
    /// The raw `wgpu::ShaderModule` — needed by `graphics-driver`'s
    /// render pipeline creation (vertex + fragment stages from the same
    /// module), which this crate's own [`Device::create_compute_pipeline`]
    /// doesn't cover.
    pub fn wgpu_shader(&self) -> &wgpu::ShaderModule {
        &self.raw
    }
}

/// A configured compute pipeline (shader + bind group layout).
#[derive(Debug)]
pub struct ComputePipeline {
    raw: wgpu::ComputePipeline,
}

impl ComputePipeline {
    /// The bind group layout `Device::create_compute_pipeline` derived
    /// from the shader — an escape hatch for callers building their own
    /// bind groups beyond [`Device::create_single_buffer_bind_group`]'s
    /// single-binding shape.
    pub fn bind_group_layout(&self) -> wgpu::BindGroupLayout {
        self.raw.get_bind_group_layout(0)
    }

    /// The raw `wgpu::ComputePipeline` — an escape hatch for callers
    /// recording their own compute pass with a bind group shape beyond
    /// [`Device::create_single_buffer_bind_group`]'s single-binding one
    /// (see [`CommandBuffer::encoder_mut`]'s doc comment for the same
    /// pattern applied to command recording).
    pub fn wgpu_pipeline(&self) -> &wgpu::ComputePipeline {
        &self.raw
    }
}

/// A bound resource group — today, always the single-buffer shape
/// [`Device::create_single_buffer_bind_group`] builds.
#[derive(Debug)]
pub struct BindGroup {
    raw: wgpu::BindGroup,
}

impl BindGroup {
    pub fn wgpu_bind_group(&self) -> &wgpu::BindGroup {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test here needs a real adapter; some CI/sandboxed
    /// environments have none (no GPU, no software rasterizer ICD
    /// installed). Skip rather than fail in that case — this validates
    /// this crate's own plumbing, not that a GPU is present.
    async fn device_or_skip() -> Option<Device> {
        match Device::new().await {
            Ok(device) => Some(device),
            Err(err) => {
                eprintln!("skipping: no GPU device available ({err})");
                None
            }
        }
    }

    #[tokio::test]
    async fn device_reports_a_nonempty_adapter_name() {
        let Some(device) = device_or_skip().await else {
            return;
        };
        assert!(!device.adapter_name().is_empty());
    }

    #[tokio::test]
    async fn device_reports_gpu_capabilities_matching_adapter_name() {
        let Some(device) = device_or_skip().await else {
            return;
        };
        let gpu = device
            .gpu()
            .expect("a constructed Device always has a real adapter");
        assert_eq!(gpu.device_name, device.adapter_name());
        assert!(device.gpu_available());
    }

    #[tokio::test]
    async fn buffer_write_then_read_round_trips() {
        let Some(device) = device_or_skip().await else {
            return;
        };
        let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let buffer = device.create_buffer(data.len(), BufferUsage::Storage);
        device.write_buffer(&buffer, &data);
        let read_back = device.read_buffer(&buffer).await;
        assert_eq!(read_back, data);
    }

    #[tokio::test]
    async fn texture_reports_requested_dimensions() {
        let Some(device) = device_or_skip().await else {
            return;
        };
        let texture = device.create_texture(
            64,
            32,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
        );
        assert_eq!(texture.width, 64);
        assert_eq!(texture.height, 32);
    }

    /// The actual end-to-end proof this module exists to give: write
    /// data to a buffer, run a real compute shader over it on the GPU,
    /// read the result back, and check it matches what the shader says
    /// it does — not just "didn't panic".
    #[tokio::test]
    async fn compute_dispatch_doubles_every_element() {
        let Some(device) = device_or_skip().await else {
            return;
        };

        const SHADER: &str = r#"
            @group(0) @binding(0)
            var<storage, read_write> data: array<u32>;

            @compute @workgroup_size(4)
            fn double_elements(@builtin(global_invocation_id) id: vec3<u32>) {
                data[id.x] = data[id.x] * 2u;
            }
        "#;

        let input: [u32; 4] = [1, 2, 3, 4];
        let bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();

        let buffer = device.create_buffer(bytes.len(), BufferUsage::Storage);
        device.write_buffer(&buffer, &bytes);

        let shader = device.create_shader("double_elements", SHADER);
        let pipeline = device.create_compute_pipeline(&shader, "double_elements");

        let mut commands = device.create_command_buffer();
        commands.dispatch_compute(&pipeline, &buffer, 1);
        commands.submit();

        let result_bytes = device.read_buffer(&buffer).await;
        let result: Vec<u32> = result_bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(result, vec![2, 4, 6, 8]);
    }
}
