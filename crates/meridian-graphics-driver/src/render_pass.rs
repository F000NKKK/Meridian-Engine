//! One render pass being recorded.

use crate::RenderPipeline;

/// A single recorded render pass, opened by
/// [`CommandBuffer::begin_render_pass`](crate::CommandBuffer::begin_render_pass). Ends (and its draw calls become
/// part of the owning [`CommandBuffer`](crate::CommandBuffer)) when dropped.
#[derive(Debug)]
pub struct RenderPass<'pass> {
    pub(crate) raw: wgpu::RenderPass<'pass>,
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
