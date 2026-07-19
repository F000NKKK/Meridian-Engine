//! Low-level GPU abstraction (device, command queues, buffers, textures, shaders, pipelines, synchronization). Knows nothing about scenes or materials.
//!
//! Real, backed by `wgpu` (see docs/roadmap.md "Not yet decided" for why:
//! Vulkan/DX12/Metal in one safe API, not hand-written per-backend FFI).
//! **Headless only, for now** — [`Device::new`] requests an adapter with
//! no `compatible_surface`, so there is no window/swapchain here yet;
//! that's windowing's own concrete decision (`platform-core::Window` is
//! still a stub) and a separate follow-up, not blocked on anything in
//! this module. `wgpu`'s device/adapter acquisition is `async`;
//! [`pollster::block_on`] bridges that to this crate's otherwise-sync API
//! at the one point it's unavoidable, rather than making every caller in
//! the workspace `async` for a single GPU handshake (see roadmap.md's
//! `wgpu` decision for the same reasoning).
//!
//! [`Pipeline`] is compute-only so far: a render pipeline needs a vertex
//! layout and color-target formats, which need either a real swapchain
//! surface or a vocabulary for meshes/materials that doesn't exist yet
//! (`graphics-core`'s job, not this driver's) — see docs/roadmap.md.
//! Everything else (`Device`, `CommandBuffer`, `Buffer`, `Texture`,
//! `Shader`) is fully real and exercised end-to-end by this module's own
//! tests: write a buffer, run a compute shader over it, read the result
//! back.

use std::sync::mpsc;

use meridian_platform_core::{BackendCapabilities, CpuCapabilities, GpuCapabilities};

/// A GPU device: owns the `wgpu::Device`/`wgpu::Queue` pair every other
/// type here is created through or submitted against.
#[derive(Debug)]
pub struct Device {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
}

/// Why [`Device::new`] failed. Both variants are real, reachable
/// failures — not "shouldn't happen" cases — since adapter/device
/// acquisition depends on what's actually installed on the machine
/// running this code (driver, ICD, `wgpu` backend availability).
#[derive(Debug)]
pub enum DeviceError {
    /// No adapter matched the request (no GPU, no compatible driver, or
    /// `wgpu` has no backend for this platform).
    NoAdapter,
    RequestDevice(wgpu::RequestDeviceError),
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceError::NoAdapter => write!(f, "no compatible GPU adapter found"),
            DeviceError::RequestDevice(e) => write!(f, "failed to request GPU device: {e}"),
        }
    }
}

impl std::error::Error for DeviceError {}

impl Device {
    /// Requests a headless (no-surface) GPU device: an `Instance` with
    /// every backend `wgpu` supports on this platform enabled, the
    /// adapter it picks (`compatible_surface: None` — this is the
    /// headless path, see the module doc), then the logical device off
    /// that adapter. Blocking, via [`pollster::block_on`] — see the
    /// module doc for why that's the deliberate bridge point rather than
    /// this crate's API being `async`.
    pub fn new() -> Result<Self, DeviceError> {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Result<Self, DeviceError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await
            .map_err(|_| DeviceError::NoAdapter)?;
        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("meridian-graphics-driver device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await
            .map_err(DeviceError::RequestDevice)?;
        Ok(Self {
            device,
            queue,
            adapter_info,
        })
    }

    /// The adapter's reported name (e.g. `"NVIDIA GeForce RTX 4050
    /// Max-Q"`) — for logging/diagnostics, not parsed for behavior.
    pub fn adapter_name(&self) -> &str {
        &self.adapter_info.name
    }

    /// Allocates a GPU-visible buffer of `byte_len` bytes. Always usable
    /// as a copy source/destination in addition to `usage`'s role — a
    /// generous default rather than fine-grained per-buffer usage-flag
    /// control, which is future work once a concrete need for tighter
    /// flags shows up (matches this workspace's general policy of
    /// correct-and-simple first, optimized later once profiling/need
    /// calls for it).
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

    /// Reads `buffer`'s entire contents back to the CPU. Blocking: copies
    /// into a `MAP_READ`-capable staging buffer (most `buffer` usages,
    /// e.g. `STORAGE`, aren't directly mappable on discrete GPUs), submits
    /// that copy, then waits on the map — the standard `wgpu` readback
    /// pattern, not a `Buffer`-type-specific hack.
    pub fn read_buffer(&self, buffer: &Buffer) -> Vec<u8> {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("meridian-graphics-driver readback staging buffer"),
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
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
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

    /// A 2D `Rgba8Unorm` texture usable as a copy destination and a
    /// shader binding — the common case; other formats/usages are future
    /// work once a concrete consumer needs them.
    pub fn create_texture(&self, width: u32, height: u32) -> Texture {
        let raw = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Texture { raw, width, height }
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
    pub fn create_compute_pipeline(&self, shader: &Shader, entry_point: &str) -> Pipeline {
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
        Pipeline { raw }
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
    /// `graphics-driver` has no CPU-dispatch path of its own (unlike
    /// `compute-driver`/`physics-driver`) — this reports the machine's CPU
    /// thread count for consistency with every other `*-driver`'s
    /// `BackendCapabilities` impl, not because `Device` schedules any CPU
    /// work across them.
    fn cpu(&self) -> CpuCapabilities {
        CpuCapabilities::detect()
    }

    /// Always `Some` — a `Device` only exists once [`Device::new`] has
    /// already found a real adapter, unlike `compute-driver`/
    /// `physics-driver`/`audio-driver`, which report `None` because they
    /// have no GPU backend to ask at all.
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

impl<'a> CommandBuffer<'a> {
    /// Records a compute dispatch: binds `buffer` at `@group(0)
    /// @binding(0)` (the only binding shape supported so far — see
    /// [`Device::create_compute_pipeline`]'s doc comment) and dispatches
    /// `workgroups` workgroups along X (Y/Z left at 1 — 1D dispatch is
    /// all today's callers need; extending to 3D is additive).
    pub fn dispatch_compute(&mut self, pipeline: &Pipeline, buffer: &Buffer, workgroups: u32) {
        let bind_group_layout = pipeline.raw.get_bind_group_layout(0);
        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.raw.as_entire_binding(),
                }],
            });

        let mut pass = self
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline.raw);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
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

/// A GPU texture resource. `raw` isn't read anywhere yet — no
/// `Device` method uploads/reads texture data or binds one to a
/// pipeline today (`create_texture` only allocates it) — but it holds
/// the real `wgpu::Texture` handle so those operations are additive
/// later, not a redesign.
#[derive(Debug)]
pub struct Texture {
    #[allow(dead_code)]
    raw: wgpu::Texture,
    pub width: u32,
    pub height: u32,
}

/// A compiled shader module.
#[derive(Debug)]
pub struct Shader {
    raw: wgpu::ShaderModule,
}

/// A configured GPU pipeline (shaders + fixed-function state).
/// Compute-only so far — see the module doc.
#[derive(Debug)]
pub struct Pipeline {
    raw: wgpu::ComputePipeline,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test here needs a real adapter; some CI/sandboxed
    /// environments have none (no GPU, no software rasterizer ICD
    /// installed). Skip rather than fail in that case — this validates
    /// the driver's own plumbing, not that a GPU is present, and a
    /// missing GPU isn't this crate's bug.
    fn device_or_skip() -> Option<Device> {
        match Device::new() {
            Ok(device) => Some(device),
            Err(err) => {
                eprintln!("skipping: no GPU device available ({err})");
                None
            }
        }
    }

    #[test]
    fn device_reports_a_nonempty_adapter_name() {
        let Some(device) = device_or_skip() else {
            return;
        };
        assert!(!device.adapter_name().is_empty());
    }

    #[test]
    fn device_reports_gpu_capabilities_matching_adapter_name() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let gpu = device
            .gpu()
            .expect("a constructed Device always has a real adapter");
        assert_eq!(gpu.device_name, device.adapter_name());
        assert!(device.gpu_available());
    }

    #[test]
    fn buffer_write_then_read_round_trips() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let buffer = device.create_buffer(data.len(), BufferUsage::Storage);
        device.write_buffer(&buffer, &data);
        let read_back = device.read_buffer(&buffer);
        assert_eq!(read_back, data);
    }

    #[test]
    fn texture_reports_requested_dimensions() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let texture = device.create_texture(64, 32);
        assert_eq!(texture.width, 64);
        assert_eq!(texture.height, 32);
    }

    /// The actual end-to-end proof this module exists to give: write
    /// data to a buffer, run a real compute shader over it on the GPU,
    /// read the result back, and check it matches what the shader says
    /// it does — not just "didn't panic".
    #[test]
    fn compute_dispatch_doubles_every_element() {
        let Some(device) = device_or_skip() else {
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

        let result_bytes = device.read_buffer(&buffer);
        let result: Vec<u32> = result_bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(result, vec![2, 4, 6, 8]);
    }
}
