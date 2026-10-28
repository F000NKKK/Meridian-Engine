//! Fullscreen post-process pipeline creation on the device.

use crate::{ColorFormat, Device, RenderPipeline, Surface};

const ADDITIVE_COMPONENT: wgpu::BlendComponent = wgpu::BlendComponent {
    src_factor: wgpu::BlendFactor::One,
    dst_factor: wgpu::BlendFactor::One,
    operation: wgpu::BlendOperation::Add,
};

const ADDITIVE: wgpu::BlendState = wgpu::BlendState {
    color: ADDITIVE_COMPONENT,
    alpha: ADDITIVE_COMPONENT,
};

/// What a full-screen pipeline draws into.
struct FullscreenTarget {
    format: wgpu::TextureFormat,
    additive: bool,
    depth_stencil: Option<wgpu::DepthStencilState>,
}

impl FullscreenTarget {
    fn new(format: wgpu::TextureFormat, additive: bool) -> Self {
        Self {
            format,
            additive,
            depth_stencil: None,
        }
    }
}

impl Device {
    /// Builds a full-screen-triangle pipeline: no vertex buffer (three
    /// vertices generated in `shader` from `@builtin(vertex_index)`, the
    /// standard `wgpu` full-screen-triangle trick — one triangle
    /// covering the whole clip-space rectangle, cheaper than a
    /// two-triangle quad and with no seam), no depth test, and `blend`
    /// picking `REPLACE` (a filter pass, e.g. blur) or `ADD` (a bloom-
    /// style additive composite onto whatever the target already holds
    /// — see [`CommandBuffer::begin_render_pass_loaded`](crate::CommandBuffer::begin_render_pass_loaded)). Used for
    /// post-process passes; ordinary mesh rendering stays on
    /// [`create_render_pipeline`](Self::create_render_pipeline).
    pub fn create_fullscreen_pipeline(
        &self,
        shader: &meridian_gpu_wgpu::Shader,
        fragment_entry: &str,
        surface: &Surface,
        additive: bool,
    ) -> RenderPipeline {
        self.create_fullscreen_pipeline_for_format(
            shader,
            fragment_entry,
            FullscreenTarget::new(surface.config.format, additive),
        )
    }

    /// [`create_fullscreen_pipeline`](Self::create_fullscreen_pipeline)'s
    /// offscreen counterpart: targets `format` directly — pass whatever
    /// [`ColorFormat`](crate::ColorFormat) the offscreen texture this pipeline draws into
    /// (see [`create_offscreen_color_texture`](Self::create_offscreen_color_texture))
    /// actually used, so the two can never silently drift apart the way
    /// two independent hardcoded formats could.
    pub fn create_fullscreen_pipeline_for_offscreen(
        &self,
        shader: &meridian_gpu_wgpu::Shader,
        fragment_entry: &str,
        format: ColorFormat,
        additive: bool,
    ) -> RenderPipeline {
        self.create_fullscreen_pipeline_for_format(
            shader,
            fragment_entry,
            FullscreenTarget::new(format.to_wgpu(), additive),
        )
    }

    /// A full-screen-triangle pipeline for the backdrop of the main scene pass
    /// (ADR 039): it declares the pass's depth attachment but never tests
    /// against or writes it, so the geometry drawn after it lands over the sky
    /// whatever its depth.
    #[must_use]
    pub fn create_sky_pipeline(
        &self,
        shader: &meridian_gpu_wgpu::Shader,
        fragment_entry: &str,
        format: ColorFormat,
    ) -> RenderPipeline {
        let depth = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };
        self.create_fullscreen_pipeline_for_format(
            shader,
            fragment_entry,
            FullscreenTarget {
                depth_stencil: Some(depth),
                ..FullscreenTarget::new(format.to_wgpu(), false)
            },
        )
    }

    fn create_fullscreen_pipeline_for_format(
        &self,
        shader: &meridian_gpu_wgpu::Shader,
        fragment_entry: &str,
        target: FullscreenTarget,
    ) -> RenderPipeline {
        let FullscreenTarget {
            format,
            additive,
            depth_stencil,
        } = target;
        let blend = if additive {
            ADDITIVE
        } else {
            wgpu::BlendState::REPLACE
        };
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
                        blend: Some(blend),
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
                depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        RenderPipeline { raw }
    }
}
