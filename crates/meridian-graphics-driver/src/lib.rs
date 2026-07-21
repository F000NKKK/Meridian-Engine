//! Low-level GPU abstraction (device, command queues, buffers, textures, shaders, pipelines, synchronization). Knows nothing about scenes or materials.
//!
//! Real, backed by `wgpu` via [`meridian_gpu_driver`] — the crate shared
//! with `compute-driver` that owns the actual wgpu device/buffer/shader
//! mechanics (see that crate's module doc and
//! [ADR 011](../../../docs/adr/011-shared-gpu-driver-crate.md)); this
//! crate adds only what's graphics-specific on top: [`Surface`]
//! (swapchain configuration/lifecycle), [`RenderPipeline`]
//! (vertex+fragment, with an optional [`DepthTexture`]), and
//! [`RenderPass`] recording. General GPU *compute* dispatch (a compute
//! pipeline bound to a buffer, unrelated to rendering) is
//! `compute-driver`'s job now, not this crate's — see that crate for it.
//!
//! Both headless ([`Device::new`], no `compatible_surface`) and windowed
//! ([`Device::new_windowed`], a real swapchain [`Surface`]) construction
//! exist. This crate stays `winit`-agnostic even for the windowed path —
//! [`Device::new_windowed`] takes `impl Into<wgpu::SurfaceTarget<'static>>`,
//! a `wgpu`-defined bound `Arc<winit::window::Window>` already satisfies
//! — see `platform-core::Window::surface_target` and
//! [ADR 010](../../../docs/adr/010-windowing-via-winit.md).
//!
//! **Async on genuine I/O, not on everything** — see
//! [ADR 009](../../../docs/adr/009-async-io-via-tokio.md) and
//! `meridian_gpu_driver`'s module doc for the same policy this crate
//! inherits: [`Device::new`]/[`Device::new_windowed`]/[`Device::read_buffer`]
//! are real `async fn`s; recording/allocation calls stay synchronous.
//!
//! Everything here is exercised end-to-end by the `spinning_cube`
//! example (windowed render path); `meridian_gpu_driver`'s own tests
//! cover the shared device/buffer/shader mechanics (headless compute
//! path).

use meridian_platform_core::{BackendCapabilities, CpuCapabilities, GpuCapabilities};

// `Buffer`/`BindGroup` are re-exported so callers that already reach
// this crate's API (which returns/consumes them directly, e.g.
// `create_buffer`/`create_uniform_bind_group`) never need their own
// edge to `gpu-driver` just to name the type — the same
// resource-type-naming precedent as `gac-compute`'s direct `gpu-driver`
// dependency (ADR 011), applied by re-export instead of a second edge.
pub use meridian_gpu_driver::{BindGroup, Buffer, BufferUsage, DeviceError, Sampler, Texture};

/// A windowed-capable GPU device. Wraps [`meridian_gpu_driver::Device`]
/// (which owns the actual `wgpu::Device`/`wgpu::Queue` and every
/// buffer/shader mechanic) and adds nothing but the graphics-specific
/// constructors/resources declared in this crate — buffer/shader/texture
/// methods forward directly to the inner device, not reimplemented here.
#[derive(Debug)]
pub struct Device(meridian_gpu_driver::Device);

impl Device {
    /// Requests a headless (no-surface) GPU device — see
    /// [`meridian_gpu_driver::Device::new`].
    pub async fn new() -> Result<Self, DeviceError> {
        Ok(Self(meridian_gpu_driver::Device::new().await?))
    }

    /// Requests a windowed GPU device plus its swapchain [`Surface`],
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

    /// A `Depth32Float` depth/stencil-comparable texture (full float
    /// precision, no stencil since nothing here uses one yet), for
    /// [`CommandBuffer::begin_render_pass`]'s `depth` parameter.
    pub fn create_depth_texture(&self, width: u32, height: u32) -> DepthTexture {
        let texture = self.0.create_texture(
            width,
            height,
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        DepthTexture { texture }
    }

    /// An `Rgba8UnormSrgb`, sampleable color texture — the shape a
    /// decoded `asset-core::ImageData` (already RGBA8) uploads into for
    /// use as a material's albedo. sRGB, matching this crate's swapchain
    /// format choice (see [`Device::new_windowed`]'s doc comment), so a
    /// texture authored in sRGB (almost every image file is) reads back
    /// linear in the shader without a manual gamma-correction step.
    pub fn create_texture_2d(&self, width: u32, height: u32) -> Texture {
        self.0.create_texture(
            width,
            height,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        )
    }

    /// Uploads tightly-packed RGBA8 pixel data to `texture`'s full
    /// extent — see [`meridian_gpu_driver::Device::write_texture`].
    pub fn write_texture(&self, texture: &Texture, rgba8_data: &[u8]) {
        self.0.write_texture(texture, rgba8_data);
    }

    /// A linear-filtered, repeat-addressed sampler (for tiled surface
    /// UVs) — see [`meridian_gpu_driver::Device::create_sampler`].
    pub fn create_sampler(&self) -> Sampler {
        self.0.create_sampler()
    }

    /// A linear-filtered, clamp-to-edge sampler (for full-screen post-
    /// process passes) — see
    /// [`meridian_gpu_driver::Device::create_clamp_sampler`].
    pub fn create_clamp_sampler(&self) -> Sampler {
        self.0.create_clamp_sampler()
    }

    /// Builds a bind group binding `buffer` (a uniform, as in
    /// [`create_uniform_bind_group`](Self::create_uniform_bind_group)) at
    /// binding 0, `texture`'s view at binding 1 and `sampler` at binding
    /// 2 of `pipeline`'s auto-derived layout — the shape a shader that
    /// samples one albedo texture alongside its view-projection uniform
    /// needs.
    pub fn create_textured_bind_group(
        &self,
        pipeline: &RenderPipeline,
        buffer: &Buffer,
        texture: &Texture,
        sampler: &Sampler,
    ) -> BindGroup {
        self.0.create_texture_bind_group(
            &pipeline.raw.get_bind_group_layout(0),
            buffer,
            texture,
            sampler,
        )
    }

    /// Builds a render pipeline: `vertex_entry`/`fragment_entry` are two
    /// entry points in the same `shader` module (a WGSL convention, not a
    /// `wgpu` requirement — nothing stops a caller passing the same
    /// `Shader` twice with different entry point names). `vertex_layout`
    /// describes one vertex buffer's attributes (see [`VertexLayout`]);
    /// `surface` supplies the color target format so the pipeline matches
    /// what it will actually render into; `depth_enabled` adds a
    /// `Depth32Float` depth-test/write stage matching
    /// [`Device::create_depth_texture`]'s format. The bind group layout
    /// is auto-derived from the shader's own `@group`/`@binding`
    /// declarations rather than hand-specified — correct and simple for
    /// the single binding-per-draw shape [`Device::create_uniform_bind_group`]
    /// supports today; an explicit layout becomes worth building once a
    /// pipeline needs more than one bound resource.
    pub fn create_render_pipeline(
        &self,
        shader: &meridian_gpu_driver::Shader,
        vertex_entry: &str,
        fragment_entry: &str,
        vertex_layout: &VertexLayout,
        surface: &Surface,
        depth_enabled: bool,
    ) -> RenderPipeline {
        self.create_render_pipeline_for_format(
            shader,
            vertex_entry,
            fragment_entry,
            vertex_layout,
            surface.config.format,
            depth_enabled,
        )
    }

    /// [`create_render_pipeline`](Self::create_render_pipeline)'s
    /// offscreen counterpart: targets `format` directly — pass whatever
    /// [`ColorFormat`] the offscreen texture this pipeline draws into
    /// actually used (see [`create_offscreen_color_texture`](Self::create_offscreen_color_texture)),
    /// so the two can never silently drift apart (see [`ColorFormat`]'s
    /// own doc comment).
    pub fn create_render_pipeline_for_offscreen(
        &self,
        shader: &meridian_gpu_driver::Shader,
        vertex_entry: &str,
        fragment_entry: &str,
        vertex_layout: &VertexLayout,
        format: ColorFormat,
        depth_enabled: bool,
    ) -> RenderPipeline {
        self.create_render_pipeline_for_format(
            shader,
            vertex_entry,
            fragment_entry,
            vertex_layout,
            format.to_wgpu(),
            depth_enabled,
        )
    }

    /// General form taking the target color format directly instead of
    /// reading it off a [`Surface`] or a [`ColorFormat`] — the primitive
    /// [`create_render_pipeline`](Self::create_render_pipeline) and
    /// [`create_render_pipeline_for_offscreen`](Self::create_render_pipeline_for_offscreen)
    /// both delegate to.
    pub fn create_render_pipeline_for_format(
        &self,
        shader: &meridian_gpu_driver::Shader,
        vertex_entry: &str,
        fragment_entry: &str,
        vertex_layout: &VertexLayout,
        format: wgpu::TextureFormat,
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
            .0
            .wgpu_device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: wgpu::VertexState {
                    module: shader.wgpu_shader(),
                    entry_point: Some(vertex_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(buffer_layout)],
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader.wgpu_shader(),
                    entry_point: Some(fragment_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
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

    /// A sampleable *and* renderable color texture
    /// (`TEXTURE_BINDING | RENDER_ATTACHMENT`, in `format`) — the shape a
    /// post-process pass's intermediate target needs: something to
    /// render into, then read back from in the next pass.
    /// [`create_texture_2d`](Self::create_texture_2d) is sampleable but
    /// not renderable (an uploaded asset texture is never a render
    /// target); this is that plus render-attachment usage.
    pub fn create_offscreen_color_texture(
        &self,
        width: u32,
        height: u32,
        format: ColorFormat,
    ) -> Texture {
        self.0.create_texture(
            width,
            height,
            format.to_wgpu(),
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
    }

    /// Builds a full-screen-triangle pipeline: no vertex buffer (three
    /// vertices generated in `shader` from `@builtin(vertex_index)`, the
    /// standard `wgpu` full-screen-triangle trick — one triangle
    /// covering the whole clip-space rectangle, cheaper than a
    /// two-triangle quad and with no seam), no depth test, and `blend`
    /// picking `REPLACE` (a filter pass, e.g. blur) or `ADD` (a bloom-
    /// style additive composite onto whatever the target already holds
    /// — see [`CommandBuffer::begin_render_pass_loaded`]). Used for
    /// post-process passes; ordinary mesh rendering stays on
    /// [`create_render_pipeline`](Self::create_render_pipeline).
    pub fn create_fullscreen_pipeline(
        &self,
        shader: &meridian_gpu_driver::Shader,
        fragment_entry: &str,
        surface: &Surface,
        additive: bool,
    ) -> RenderPipeline {
        self.create_fullscreen_pipeline_for_format(
            shader,
            fragment_entry,
            surface.config.format,
            additive,
        )
    }

    /// [`create_fullscreen_pipeline`](Self::create_fullscreen_pipeline)'s
    /// offscreen counterpart: targets `format` directly — pass whatever
    /// [`ColorFormat`] the offscreen texture this pipeline draws into
    /// (see [`create_offscreen_color_texture`](Self::create_offscreen_color_texture))
    /// actually used, so the two can never silently drift apart the way
    /// two independent hardcoded formats could.
    pub fn create_fullscreen_pipeline_for_offscreen(
        &self,
        shader: &meridian_gpu_driver::Shader,
        fragment_entry: &str,
        format: ColorFormat,
        additive: bool,
    ) -> RenderPipeline {
        self.create_fullscreen_pipeline_for_format(
            shader,
            fragment_entry,
            format.to_wgpu(),
            additive,
        )
    }

    fn create_fullscreen_pipeline_for_format(
        &self,
        shader: &meridian_gpu_driver::Shader,
        fragment_entry: &str,
        format: wgpu::TextureFormat,
        additive: bool,
    ) -> RenderPipeline {
        let raw = self
            .0
            .wgpu_device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: wgpu::VertexState {
                    module: shader.wgpu_shader(),
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader.wgpu_shader(),
                    entry_point: Some(fragment_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(if additive {
                            wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                            }
                        } else {
                            wgpu::BlendState::REPLACE
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        RenderPipeline { raw }
    }

    /// Builds a bind group binding `buffer` (typically a `Uniform`
    /// buffer — a per-frame MVP matrix, for instance) at `@group(0)
    /// @binding(0)` of `pipeline` — the same single-binding shape
    /// [`Device::create_render_pipeline`]'s auto-derived layout expects.
    pub fn create_uniform_bind_group(
        &self,
        pipeline: &RenderPipeline,
        buffer: &meridian_gpu_driver::Buffer,
    ) -> meridian_gpu_driver::BindGroup {
        self.0
            .create_single_buffer_bind_group(&pipeline.raw.get_bind_group_layout(0), buffer)
    }

    /// Opens a new [`CommandBuffer`] for recording. Nothing reaches the
    /// GPU until [`CommandBuffer::submit`] is called.
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

/// A recorded, submittable sequence of GPU commands. Wraps
/// [`meridian_gpu_driver::CommandBuffer`], adding
/// [`CommandBuffer::begin_render_pass`] — the graphics-specific recording
/// operation the shared crate deliberately doesn't know about.
#[derive(Debug)]
pub struct CommandBuffer<'a> {
    inner: meridian_gpu_driver::CommandBuffer<'a>,
}

impl<'a> CommandBuffer<'a> {
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
        let raw = self
            .inner
            .encoder_mut()
            .begin_render_pass(&wgpu::RenderPassDescriptor {
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
                    view: d.texture.view(),
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

    /// Like [`begin_render_pass`](Self::begin_render_pass), but preserves
    /// `color_target`'s existing contents (`LoadOp::Load`) instead of
    /// clearing them, and never adds a depth stage. For a pass that
    /// *accumulates* onto a target an earlier pass in the same command
    /// buffer already drew into — the bloom composite pass is the
    /// concrete case: it additively blends a blurred bright-pass texture
    /// onto the swapchain view the main scene pass already rendered into
    /// moments earlier in the same [`CommandBuffer`].
    pub fn begin_render_pass_loaded<'pass>(
        &'pass mut self,
        color_target: &'pass wgpu::TextureView,
    ) -> RenderPass<'pass> {
        let raw = self
            .inner
            .encoder_mut()
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        RenderPass { raw }
    }

    /// Submits every recorded command to the device's queue.
    pub fn submit(self) {
        self.inner.submit();
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

    pub fn set_bind_group(
        &mut self,
        group_index: u32,
        bind_group: &meridian_gpu_driver::BindGroup,
    ) {
        self.raw
            .set_bind_group(group_index, bind_group.wgpu_bind_group(), &[]);
    }

    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: &meridian_gpu_driver::Buffer) {
        self.raw
            .set_vertex_buffer(slot, buffer.wgpu_buffer().slice(..));
    }

    /// `u16` indices — the common case for a mesh with under 65536
    /// vertices; a `u32` variant is additive future work if a mesh ever
    /// needs more.
    pub fn set_index_buffer_u16(&mut self, buffer: &meridian_gpu_driver::Buffer) {
        self.raw
            .set_index_buffer(buffer.wgpu_buffer().slice(..), wgpu::IndexFormat::Uint16);
    }

    pub fn draw(&mut self, vertices: core::ops::Range<u32>) {
        self.raw.draw(vertices, 0..1);
    }

    pub fn draw_indexed(&mut self, indices: core::ops::Range<u32>) {
        self.raw.draw_indexed(indices, 0, 0..1);
    }
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
            AcquireFrameError::Outdated => {
                write!(f, "surface configuration is outdated, call Surface::resize")
            }
            AcquireFrameError::Lost => write!(f, "surface was lost and needs to be recreated"),
            AcquireFrameError::Validation => {
                write!(f, "validation error acquiring the next swapchain frame")
            }
        }
    }
}

impl std::error::Error for AcquireFrameError {}

impl meridian_foundation::EngineError for AcquireFrameError {}

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

    /// Presents this frame via `device`'s queue.
    pub fn present(self, device: &Device) {
        device.0.wgpu_queue().clone().present(self.output);
    }
}

/// A depth/stencil-comparable texture — see [`Device::create_depth_texture`].
#[derive(Debug)]
pub struct DepthTexture {
    texture: meridian_gpu_driver::Texture,
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
/// workspace has no use for yet) — extending this is additive as a
/// concrete need shows up.
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

/// A color texture format callers can name without depending on `wgpu`
/// directly — the same "small enum, extend additively" shape as
/// [`VertexFormat`]. Threading an explicit `ColorFormat` through
/// [`Device::create_offscreen_color_texture`] and its matching pipeline
/// constructors ([`Device::create_fullscreen_pipeline_for_offscreen`],
/// [`Device::create_render_pipeline_for_offscreen`]) is what makes an
/// offscreen texture's format and the pipeline that renders into it
/// *one* value instead of two independently-hardcoded assumptions that
/// can silently drift apart — the wgpu validation error this replaces
/// ("RenderPass uses textures with format X but the RenderPipeline
/// uses Y") is exactly what happens when they do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFormat {
    /// 8-bit sRGB RGBA — what [`Device::create_texture_2d`] and every
    /// offscreen target in this workspace uses today. An HDR variant
    /// (`Rgba16Float`) is real future work (see `bloom.rs`'s module doc
    /// in `graphics-core`), additive here when something needs it.
    SrgbRgba8,
}

impl ColorFormat {
    fn to_wgpu(self) -> wgpu::TextureFormat {
        match self {
            ColorFormat::SrgbRgba8 => wgpu::TextureFormat::Rgba8UnormSrgb,
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

/// A configured render pipeline — vertex + fragment stages, primitive/
/// culling state, and an optional depth-test stage. Built by
/// [`Device::create_render_pipeline`], bound via
/// [`RenderPass::set_pipeline`].
#[derive(Debug)]
pub struct RenderPipeline {
    raw: wgpu::RenderPipeline,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test here needs a real adapter; some CI/sandboxed
    /// environments have none. Skip rather than fail in that case.
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
    async fn texture_2d_upload_and_sampler_creation_do_not_panic() {
        let Some(device) = device_or_skip().await else {
            return;
        };
        // 2x2 RGBA8: enough to exercise write_texture's row-layout math
        // without needing a real decoded image asset.
        let pixels: [u8; 16] = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let texture = device.create_texture_2d(2, 2);
        device.write_texture(&texture, &pixels);
        let _sampler = device.create_sampler();
    }
}
