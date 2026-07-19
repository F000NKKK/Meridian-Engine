//! Low-level GPU abstraction (device, command queues, buffers, textures, shaders, pipelines, synchronization). Knows nothing about scenes or materials.
//!
//! Real, backed by `wgpu` (see docs/roadmap.md "Not yet decided" for why:
//! Vulkan/DX12/Metal in one safe API, not hand-written per-backend FFI).
//! Both headless ([`Device::new`], no `compatible_surface`) and windowed
//! ([`Device::new_windowed`], a real swapchain [`Surface`]) construction
//! exist. This crate stays `winit`-agnostic even for the windowed path —
//! [`Device::new_windowed`] takes `impl Into<wgpu::SurfaceTarget<'static>>`,
//! a `wgpu`-defined bound `Arc<winit::window::Window>` already satisfies
//! — see `platform-core::Window::surface_target` and
//! [ADR 010](../../../docs/adr/010-windowing-via-winit.md).
//!
//! **Async on genuine I/O, not on everything.** This workspace's engine
//! runs on `tokio`; every operation here whose completion time is
//! actually unbounded/externally-determined — [`Device::new`] (an OS/
//! driver handshake to find and open a GPU) and [`Device::read_buffer`]
//! (waiting for in-flight GPU work to finish) — is a real `async fn`, not
//! a blocking call hidden behind `pollster`. Recording/allocation calls
//! (`create_buffer`, `create_texture`, `create_shader`,
//! `create_compute_pipeline`, `write_buffer`, `CommandBuffer::submit`,
//! ...) stay plain synchronous functions: they're local
//! validation-and-enqueue work with bounded, effectively-instant cost —
//! the same reason `Vec::push` isn't `async` — so making them `async`
//! would add executor overhead for no benefit, not more correctness.
//! [`Device::read_buffer`] still has to manually pump `wgpu::Device::poll`
//! to drive its `map_async` callback (`wgpu` has no built-in reactor
//! integration), which is itself a blocking call — done via
//! `tokio::task::spawn_blocking` so it can't stall the async runtime's
//! worker threads while it waits.
//!
//! [`Pipeline`] (compute) and [`RenderPipeline`] (vertex+fragment, with an
//! optional depth buffer via [`DepthTexture`]) are both real. Bind groups
//! are deliberately the simplest shape that's actually useful today —
//! one bound resource at `@group(0) @binding(0)` — for both kinds of
//! pipeline (see [`CommandBuffer::dispatch_compute`] and
//! [`Device::create_uniform_bind_group`]); a general multi-binding bind
//! group builder is future work once a concrete pipeline needs more than
//! one bound resource. Meshes/materials/scene vocabulary is
//! `graphics-core`'s job, not this driver's — see docs/roadmap.md.
//! Everything here is exercised end-to-end by this module's own tests
//! (headless compute path) and by the `spinning_cube` example (windowed
//! render path).

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
    /// adapter it picks (`compatible_surface: None` — this is the
    /// headless path, see the module doc), then the logical device off
    /// that adapter. A real `async fn` — adapter/device acquisition is an
    /// OS/driver handshake with genuinely unbounded latency, exactly the
    /// kind of operation this workspace's `tokio`-based engine keeps
    /// `async` rather than blocking a worker thread on.
    pub async fn new() -> Result<Self, DeviceError> {
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

    /// Requests a windowed GPU device plus its swapchain [`Surface`],
    /// already configured at `width`x`height`. `target` is anything
    /// `wgpu` can build a surface from — in practice always
    /// `Arc<winit::window::Window>` via `platform_core::Window::surface_target`,
    /// but this crate never names `winit`'s type itself (see the module
    /// doc). The adapter is requested with `compatible_surface:
    /// Some(&surface)` (unlike [`Device::new`]'s headless path) so the
    /// chosen adapter is guaranteed able to present to this specific
    /// surface. Picks the first sRGB-capable format the surface reports
    /// (falling back to whatever format is first if none is sRGB) — sRGB
    /// output is the conventional default for correct-looking color
    /// without every shader hand-rolling its own gamma correction.
    pub async fn new_windowed(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<(Self, Surface), DeviceError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance
            .create_surface(target)
            .map_err(DeviceError::CreateSurface)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await
            .map_err(|_| DeviceError::NoAdapter)?;
        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("meridian-graphics-driver windowed device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await
            .map_err(DeviceError::RequestDevice)?;

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
        surface.configure(&device, &config);

        let device = Self {
            device,
            queue,
            adapter_info,
        };
        Ok((
            device,
            Surface {
                raw: surface,
                config,
            },
        ))
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

    /// Reads `buffer`'s entire contents back to the CPU. A real `async
    /// fn`: waiting for in-flight GPU work to finish (the copy below, and
    /// whatever else was already queued ahead of it) has genuinely
    /// unbounded latency, the same class of operation as
    /// [`Device::new`] — not local, bounded-cost work like
    /// `create_buffer`/`write_buffer`. Copies into a `MAP_READ`-capable
    /// staging buffer first (most `buffer` usages, e.g. `STORAGE`, aren't
    /// directly mappable on discrete GPUs) — the standard `wgpu` readback
    /// pattern, not a `Buffer`-type-specific hack. `wgpu` has no reactor
    /// of its own to drive `map_async`'s completion callback — something
    /// has to call `wgpu::Device::poll`, which itself blocks the calling
    /// thread until the GPU catches up — so that poll runs inside
    /// [`tokio::task::spawn_blocking`] rather than on this `async fn`'s
    /// own task, so it can't stall other work sharing the runtime's
    /// worker threads while it waits.
    pub async fn read_buffer(&self, buffer: &Buffer) -> Vec<u8> {
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
        let (tx, rx) = tokio::sync::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        let device = self.device.clone();
        tokio::task::spawn_blocking(move || device.poll(wgpu::PollType::wait_indefinitely()))
            .await
            .expect("device poll task panicked")
            .expect("device poll failed");
        rx.await
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

    /// A depth/stencil-comparable texture (`Depth32Float`, the standard
    /// choice — full float precision, no stencil since nothing here uses
    /// one yet) sized to match a [`Surface`], for
    /// [`CommandBuffer::begin_render_pass`]'s `depth` parameter.
    pub fn create_depth_texture(&self, width: u32, height: u32) -> DepthTexture {
        let raw = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("meridian-graphics-driver depth texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = raw.create_view(&wgpu::TextureViewDescriptor::default());
        DepthTexture { raw, view }
    }

    /// Builds a render pipeline: `vertex_entry`/`fragment_entry` are two
    /// entry points in the same `shader` module (a WGSL convention, not a
    /// `wgpu` requirement — nothing stops a caller passing the same
    /// `Shader` twice with different entry point names). `vertex_layout`
    /// describes one vertex buffer's attributes (see [`VertexLayout`]);
    /// `surface` supplies the color target format so the pipeline matches
    /// what it will actually render into; `depth_enabled` adds a
    /// `Depth32Float` depth-test/write stage matching
    /// [`Device::create_depth_texture`]'s format. Like
    /// [`Device::create_compute_pipeline`], the bind group layout is
    /// auto-derived from the shader's own `@group`/`@binding`
    /// declarations rather than hand-specified — see that method's doc
    /// comment for why, and [`Device::create_uniform_bind_group`] for the
    /// matching bind group constructor.
    pub fn create_render_pipeline(
        &self,
        shader: &Shader,
        vertex_entry: &str,
        fragment_entry: &str,
        vertex_layout: &VertexLayout,
        surface: &Surface,
        depth_enabled: bool,
    ) -> RenderPipeline {
        let wgpu_attributes: Vec<wgpu::VertexAttribute> = vertex_layout
            .attributes
            .iter()
            .map(|a| wgpu::VertexAttribute {
                format: a.format.to_wgpu(),
                offset: a.offset,
                shader_location: a.location,
            })
            .collect();
        let buffer_layout = wgpu::VertexBufferLayout {
            array_stride: vertex_layout.stride,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu_attributes,
        };

        let raw = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader.raw,
                    entry_point: Some(vertex_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(buffer_layout)],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader.raw,
                    entry_point: Some(fragment_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface.config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: depth_enabled.then(|| wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        RenderPipeline { raw }
    }

    /// Builds a bind group binding `buffer` (typically a `Uniform`
    /// buffer — a per-frame MVP matrix, for instance) at `@group(0)
    /// @binding(0)` of `pipeline` — the same single-binding shape
    /// [`Device::create_render_pipeline`]'s auto-derived layout expects;
    /// see that method's doc comment.
    pub fn create_uniform_bind_group(&self, pipeline: &RenderPipeline, buffer: &Buffer) -> BindGroup {
        let layout = pipeline.raw.get_bind_group_layout(0);
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.raw.as_entire_binding(),
            }],
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

    /// Opens a render pass targeting `color_target` (typically a
    /// [`SurfaceFrame::view`]), cleared to `clear_color` (RGBA, `0.0..=1.0`)
    /// before anything is drawn, with an optional `depth` attachment
    /// (cleared to `1.0` — the far plane — matching
    /// [`Device::create_render_pipeline`]'s `depth_enabled` +
    /// `CompareFunction::Less` convention: closer geometry has a *smaller*
    /// depth value). Recording ends when the returned [`RenderPass`] is
    /// dropped.
    pub fn begin_render_pass<'pass>(
        &'pass mut self,
        color_target: &'pass wgpu::TextureView,
        clear_color: [f64; 4],
        depth: Option<&'pass DepthTexture>,
    ) -> RenderPass<'pass> {
        let [r, g, b, a] = clear_color;
        let raw = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth.map(|d| wgpu::RenderPassDepthStencilAttachment {
                view: &d.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        RenderPass { raw }
    }

    /// Submits every recorded command to the device's queue.
    pub fn submit(self) {
        self.device.queue.submit(Some(self.encoder.finish()));
    }
}

/// A single recorded render pass, opened by
/// [`CommandBuffer::begin_render_pass`]. Ends (and its draw calls become
/// part of the owning [`CommandBuffer`]) when dropped.
#[derive(Debug)]
pub struct RenderPass<'pass> {
    raw: wgpu::RenderPass<'pass>,
}

impl RenderPass<'_> {
    pub fn set_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.raw.set_pipeline(&pipeline.raw);
    }

    pub fn set_bind_group(&mut self, group_index: u32, bind_group: &BindGroup) {
        self.raw.set_bind_group(group_index, &bind_group.raw, &[]);
    }

    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: &Buffer) {
        self.raw.set_vertex_buffer(slot, buffer.raw.slice(..));
    }

    /// `u16` indices — the common case for a mesh with under 65536
    /// vertices; a `u32` variant is additive future work if a mesh ever
    /// needs more.
    pub fn set_index_buffer_u16(&mut self, buffer: &Buffer) {
        self.raw
            .set_index_buffer(buffer.raw.slice(..), wgpu::IndexFormat::Uint16);
    }

    pub fn draw(&mut self, vertices: core::ops::Range<u32>) {
        self.raw.draw(vertices, 0..1);
    }

    pub fn draw_indexed(&mut self, indices: core::ops::Range<u32>) {
        self.raw.draw_indexed(indices, 0, 0..1);
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
/// Compute-only — see [`RenderPipeline`] for the render-pass equivalent.
#[derive(Debug)]
pub struct Pipeline {
    raw: wgpu::ComputePipeline,
}

/// A configured render pipeline — vertex + fragment stages, primitive/
/// culling state, and an optional depth-test stage. Built by
/// [`Device::create_render_pipeline`], bound via
/// [`RenderPass::set_pipeline`].
#[derive(Debug)]
pub struct RenderPipeline {
    raw: wgpu::RenderPipeline,
}

/// A bound resource group — today, always the single-buffer shape
/// [`Device::create_uniform_bind_group`] builds. Bound via
/// [`RenderPass::set_bind_group`].
#[derive(Debug)]
pub struct BindGroup {
    raw: wgpu::BindGroup,
}

/// A swapchain: the sequence of textures a windowed [`Device`] presents
/// to the screen. Built by [`Device::new_windowed`]; call
/// [`Surface::resize`] whenever the window's size changes (skipping a
/// stale-sized frame is worse than briefly rendering at the old size, but
/// never configuring a `0`x`0` surface — [`Surface::resize`] guards that),
/// and [`Surface::acquire_frame`] once per frame to get something to
/// render into.
#[derive(Debug)]
pub struct Surface {
    raw: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

impl Surface {
    /// The color format frames from this surface are in — what
    /// [`Device::create_render_pipeline`]'s color target must match.
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
        self.raw.configure(&device.device, &self.config);
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

/// Why [`Surface::acquire_frame`] didn't return a frame this call. Every
/// variant is a real, expected transient condition (see
/// `wgpu::CurrentSurfaceTexture`'s own variant docs) — a caller should
/// skip the frame and try again next time, not treat this as fatal,
/// except [`AcquireFrameError::Lost`] (needs the whole surface, not just
/// a frame, recreated) which is rare enough in practice not to warrant
/// its own recovery path here yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireFrameError {
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

impl std::fmt::Display for AcquireFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcquireFrameError::Timeout => write!(f, "timed out acquiring the next swapchain frame"),
            AcquireFrameError::Occluded => write!(f, "surface is occluded (minimized or hidden)"),
            AcquireFrameError::Outdated => write!(f, "surface configuration is outdated, call Surface::resize"),
            AcquireFrameError::Lost => write!(f, "surface was lost and needs to be recreated"),
            AcquireFrameError::Validation => write!(f, "validation error acquiring the next swapchain frame"),
        }
    }
}

impl std::error::Error for AcquireFrameError {}

/// One acquired swapchain frame: render into [`SurfaceFrame::view`], then
/// [`SurfaceFrame::present`] to hand it back to the OS compositor.
#[derive(Debug)]
pub struct SurfaceFrame {
    output: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
}

impl SurfaceFrame {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Presents this frame via `device`'s queue — `wgpu` moved
    /// presentation onto `Queue::present` (it used to be a method on the
    /// surface texture itself in older `wgpu` versions), so this needs
    /// the owning [`Device`], not just `self`.
    pub fn present(self, device: &Device) {
        device.queue.present(self.output);
    }
}

/// A depth/stencil-comparable texture — see [`Device::create_depth_texture`].
#[derive(Debug)]
pub struct DepthTexture {
    #[allow(dead_code)]
    raw: wgpu::Texture,
    view: wgpu::TextureView,
}

/// One vertex attribute's shape within a [`VertexLayout`]: which
/// `@location` it binds to in the shader, its component format, and its
/// byte offset within one vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexAttributeDesc {
    pub location: u32,
    pub format: VertexFormat,
    pub offset: u64,
}

/// A vertex attribute's component format. Not an exhaustive mirror of
/// `wgpu::VertexFormat` (which has ~30 variants for formats this
/// workspace has no use for yet, e.g. packed/normalized integer types) —
/// extending this is additive as a concrete need shows up, the same
/// policy [`KeyCode`](meridian_platform_core::KeyCode) uses for its own
/// deliberately-partial enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
}

impl VertexFormat {
    fn to_wgpu(self) -> wgpu::VertexFormat {
        match self {
            VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
            VertexFormat::Float32x3 => wgpu::VertexFormat::Float32x3,
            VertexFormat::Float32x4 => wgpu::VertexFormat::Float32x4,
        }
    }
}

/// Describes one vertex buffer's layout for
/// [`Device::create_render_pipeline`]: `stride` is the byte size of one
/// vertex, `attributes` is where each field lives within it.
#[derive(Debug, Clone)]
pub struct VertexLayout {
    pub stride: u64,
    pub attributes: Vec<VertexAttributeDesc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test here needs a real adapter; some CI/sandboxed
    /// environments have none (no GPU, no software rasterizer ICD
    /// installed). Skip rather than fail in that case — this validates
    /// the driver's own plumbing, not that a GPU is present, and a
    /// missing GPU isn't this crate's bug.
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
        let texture = device.create_texture(64, 32);
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
