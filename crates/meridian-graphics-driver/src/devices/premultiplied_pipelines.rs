//! Pipelines that blend premultiplied-alpha output over the target.

use super::render_pipelines::wgpu_attributes;
use crate::{ColorFormat, Device, RenderPipeline, ShaderStages, Surface, VertexLayout};

impl Device {
    /// Builds a pipeline for 2D overlays: no depth test, no culling, and
    /// premultiplied-alpha blending (`src + dst * (1 - src.a)`, for color
    /// and alpha alike) over what the target already holds. The fragment
    /// stage must output premultiplied color (`rgb * a`). `surface`
    /// supplies the color target format.
    #[must_use]
    pub fn create_premultiplied_alpha_pipeline(
        &self,
        stages: &ShaderStages<'_>,
        vertex_layout: &VertexLayout,
        surface: &Surface,
    ) -> RenderPipeline {
        self.create_premultiplied_alpha_pipeline_for_format(
            stages,
            vertex_layout,
            surface.config.format,
        )
    }

    /// [`create_premultiplied_alpha_pipeline`](Self::create_premultiplied_alpha_pipeline)'s
    /// offscreen counterpart: targets `format`, the value the offscreen
    /// texture was created with.
    #[must_use]
    pub fn create_premultiplied_alpha_pipeline_for_offscreen(
        &self,
        stages: &ShaderStages<'_>,
        vertex_layout: &VertexLayout,
        format: ColorFormat,
    ) -> RenderPipeline {
        self.create_premultiplied_alpha_pipeline_for_format(stages, vertex_layout, format.to_wgpu())
    }

    fn create_premultiplied_alpha_pipeline_for_format(
        &self,
        stages: &ShaderStages<'_>,
        vertex_layout: &VertexLayout,
        format: wgpu::TextureFormat,
    ) -> RenderPipeline {
        let attributes = wgpu_attributes(vertex_layout);
        let buffer_layout = wgpu::VertexBufferLayout {
            array_stride: vertex_layout.stride,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &attributes,
        };
        let module = stages.shader.wgpu_shader();
        let raw = self
            .0
            .wgpu_device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some(stages.vertex_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(buffer_layout)],
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(stages.fragment_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        RenderPipeline { raw }
    }
}
