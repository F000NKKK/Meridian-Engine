//! Render pipeline creation on the device.

use crate::{ColorFormat, Device, RenderPipeline, ShaderStages, Surface, VertexLayout};

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
        shader: &meridian_gpu_driver::Shader,
        vertex_entry: &str,
        vertex_layout: &VertexLayout,
    ) -> RenderPipeline {
        let wgpu_attributes = wgpu_attributes(vertex_layout);
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
        let shader = stages.shader;
        let wgpu_attributes = wgpu_attributes(vertex_layout);
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
                    entry_point: Some(stages.vertex_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(buffer_layout)],
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader.wgpu_shader(),
                    entry_point: Some(stages.fragment_entry),
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
