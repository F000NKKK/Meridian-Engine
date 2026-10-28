//! Render pipeline creation on the device.

use meridian_rendering_domain::DrawState;

use crate::{ColorFormat, Device, RenderPipeline, ShaderStages, Surface, VertexLayout, VertexStep};

impl Device {
    /// Builds a depth-only pipeline — no fragment stage, no color
    /// target, just the vertex shader writing depth — for
    /// [`CommandBuffer::begin_shadow_pass`](crate::CommandBuffer::begin_shadow_pass)'s shadow-map render. Shares
    /// the same [`VertexLayout`](crate::VertexLayout) every mesh already uploads (position,
    /// normal, UV — the shadow pass only needs position, but reusing one
    /// vertex buffer/layout for both the main and shadow pass means a
    /// mesh's buffers are never re-uploaded in a different shape just
    /// for shadows).
    pub fn create_shadow_pipeline(
        &self,
        shader: &meridian_gpu_wgpu::Shader,
        vertex_entry: &str,
        vertex_layouts: &[VertexLayout],
    ) -> RenderPipeline {
        let attributes = buffer_attributes(vertex_layouts);
        let buffers = buffer_layouts(vertex_layouts, &attributes);
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
                    buffers: &buffers,
                },
                fragment: None,
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    // Same winding/cull convention as the main pass
                    // (back-face culling). Not front-face culling: that
                    // offset stacks with the lookup shader's own bias
                    // and leaves a visible shadow gap at contact points
                    // (peter-panning). Back-face culling with the lookup
                    // bias alone handling acne is the standard pairing.
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
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

    /// Builds a render pipeline: `vertex_entry`/`fragment_entry` are two
    /// entry points in the same `shader` module (a WGSL convention, not a
    /// `wgpu` requirement — nothing stops a caller passing the same
    /// `Shader` twice with different entry point names). `vertex_layout`
    /// describes one vertex buffer's attributes (see [`VertexLayout`](crate::VertexLayout));
    /// `surface` supplies the color target format so the pipeline matches
    /// what it will actually render into; `depth_enabled` adds a
    /// `Depth32Float` depth-test/write stage matching
    /// [`Device::create_depth_texture`](crate::Device::create_depth_texture)'s format. The bind group layout
    /// is auto-derived from the shader's own `@group`/`@binding`
    /// declarations rather than hand-specified — correct and simple for
    /// the single binding-per-draw shape [`Device::create_uniform_bind_group`](crate::Device::create_uniform_bind_group)
    /// supports today; an explicit layout becomes worth building once a
    /// pipeline needs more than one bound resource.
    pub fn create_render_pipeline(
        &self,
        stages: &ShaderStages<'_>,
        vertex_layout: &VertexLayout,
        surface: &Surface,
        depth_enabled: bool,
    ) -> RenderPipeline {
        self.create_render_pipeline_for_format(
            stages,
            vertex_layout,
            surface.config.format,
            depth_enabled,
        )
    }

    /// [`create_render_pipeline`](Self::create_render_pipeline)'s
    /// offscreen counterpart: targets `format` directly — pass whatever
    /// [`ColorFormat`](crate::ColorFormat) the offscreen texture this pipeline draws into
    /// actually used (see [`create_offscreen_color_texture`](Self::create_offscreen_color_texture)),
    /// so the two can never silently drift apart (see [`ColorFormat`](crate::ColorFormat)'s
    /// own doc comment).
    pub fn create_render_pipeline_for_offscreen(
        &self,
        stages: &ShaderStages<'_>,
        vertex_layout: &VertexLayout,
        format: ColorFormat,
        depth_enabled: bool,
    ) -> RenderPipeline {
        self.create_render_pipeline_for_format(
            stages,
            vertex_layout,
            format.to_wgpu(),
            depth_enabled,
        )
    }

    /// General form taking the target color format directly instead of
    /// reading it off a [`Surface`](crate::Surface) or a [`ColorFormat`](crate::ColorFormat) — the primitive
    /// [`create_render_pipeline`](Self::create_render_pipeline) and
    /// [`create_render_pipeline_for_offscreen`](Self::create_render_pipeline_for_offscreen)
    /// both delegate to.
    pub fn create_render_pipeline_for_format(
        &self,
        stages: &ShaderStages<'_>,
        vertex_layout: &VertexLayout,
        format: wgpu::TextureFormat,
        depth_enabled: bool,
    ) -> RenderPipeline {
        let depth = depth_enabled.then(|| depth_stencil(DrawState::Opaque));
        self.build_pipeline(
            stages,
            core::slice::from_ref(vertex_layout),
            format,
            FixedFunction {
                blend: false,
                depth_stencil: depth,
            },
        )
    }

    /// A main-pass pipeline for `state`, targeting `format` (the scene target's
    /// [`ColorFormat`]): alpha blending for the blended
    /// states, and a depth stage matching
    /// [`Device::create_depth_texture`](crate::Device::create_depth_texture)'s
    /// format whose test and write follow [`DrawState`]. The on-top states
    /// keep the depth stage (the pass has a depth attachment, so every
    /// pipeline in it must declare one) but always pass and never write.
    #[must_use]
    pub fn create_scene_pipeline(
        &self,
        stages: &ShaderStages<'_>,
        vertex_layouts: &[VertexLayout],
        format: ColorFormat,
        state: DrawState,
    ) -> RenderPipeline {
        self.build_pipeline(
            stages,
            vertex_layouts,
            format.to_wgpu(),
            FixedFunction {
                blend: state.blends(),
                depth_stencil: Some(depth_stencil(state)),
            },
        )
    }

    /// [`create_scene_pipeline`](Self::create_scene_pipeline)'s offscreen
    /// counterpart for an opaque, depth-tested pass: targets `format`, the
    /// value the offscreen texture was created with.
    pub fn create_offscreen_scene_pipeline(
        &self,
        stages: &ShaderStages<'_>,
        vertex_layouts: &[VertexLayout],
        format: ColorFormat,
    ) -> RenderPipeline {
        self.build_pipeline(
            stages,
            vertex_layouts,
            format.to_wgpu(),
            FixedFunction {
                blend: false,
                depth_stencil: Some(depth_stencil(DrawState::Opaque)),
            },
        )
    }

    fn build_pipeline(
        &self,
        stages: &ShaderStages<'_>,
        vertex_layouts: &[VertexLayout],
        format: wgpu::TextureFormat,
        fixed: FixedFunction,
    ) -> RenderPipeline {
        let shader = stages.shader;
        let attributes = buffer_attributes(vertex_layouts);
        let buffers = buffer_layouts(vertex_layouts, &attributes);

        let raw = self
            .0
            .wgpu_device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: wgpu::VertexState {
                    module: shader.wgpu_shader(),
                    entry_point: Some(stages.vertex_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader.wgpu_shader(),
                    entry_point: Some(stages.fragment_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(if fixed.blend {
                            wgpu::BlendState::ALPHA_BLENDING
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
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: fixed.depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        RenderPipeline { raw }
    }
}

/// The blend and depth settings that vary between pipelines of one shader.
struct FixedFunction {
    blend: bool,
    depth_stencil: Option<wgpu::DepthStencilState>,
}

/// The depth test and write `state` asks for.
fn depth_stencil(state: DrawState) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: Some(state.writes_depth()),
        depth_compare: Some(if state.tests_depth() {
            wgpu::CompareFunction::Less
        } else {
            wgpu::CompareFunction::Always
        }),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

/// The `wgpu` attributes of each layout, one list per buffer slot.
fn buffer_attributes(layouts: &[VertexLayout]) -> Vec<Vec<wgpu::VertexAttribute>> {
    layouts.iter().map(wgpu_attributes).collect()
}

/// One `wgpu` buffer layout per slot, borrowing `attributes`.
fn buffer_layouts<'a>(
    layouts: &[VertexLayout],
    attributes: &'a [Vec<wgpu::VertexAttribute>],
) -> Vec<Option<wgpu::VertexBufferLayout<'a>>> {
    layouts
        .iter()
        .zip(attributes)
        .map(|(layout, attributes)| {
            Some(wgpu::VertexBufferLayout {
                array_stride: layout.stride,
                step_mode: match layout.step {
                    VertexStep::Vertex => wgpu::VertexStepMode::Vertex,
                    VertexStep::Instance => wgpu::VertexStepMode::Instance,
                },
                attributes,
            })
        })
        .collect()
}

/// The `wgpu` attributes of a vertex layout.
pub(super) fn wgpu_attributes(vertex_layout: &VertexLayout) -> Vec<wgpu::VertexAttribute> {
    vertex_layout
        .attributes
        .iter()
        .map(|a| wgpu::VertexAttribute {
            format: a.format.to_wgpu(),
            offset: a.offset,
            shader_location: a.location,
        })
        .collect()
}
