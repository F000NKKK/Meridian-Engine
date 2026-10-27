//! Fullscreen post-process pipeline creation on the device.

use crate::ColorFormat;
use crate::Device;
use crate::RenderPipeline;
use crate::Surface;

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
    /// [`ColorFormat`](crate::ColorFormat) the offscreen texture this pipeline draws into
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
}
