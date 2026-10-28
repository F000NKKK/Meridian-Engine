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

    pub fn set_bind_group(&mut self, group_index: u32, bind_group: &meridian_gpu_wgpu::BindGroup) {
        self.raw
            .set_bind_group(group_index, bind_group.wgpu_bind_group(), &[]);
    }

    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: &meridian_gpu_wgpu::Buffer) {
        self.raw
            .set_vertex_buffer(slot, buffer.wgpu_buffer().slice(..));
    }

    /// `u32` indices: every mesh index buffer uses them, so no mesh has a
    /// vertex-count ceiling.
    pub fn set_index_buffer_u32(&mut self, buffer: &meridian_gpu_wgpu::Buffer) {
        self.raw
            .set_index_buffer(buffer.wgpu_buffer().slice(..), wgpu::IndexFormat::Uint32);
    }

    /// Restricts rasterization to a rectangle of the target, in pixels
    /// with the origin at the top-left. Persists until set again.
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.raw.set_scissor_rect(x, y, width, height);
    }

    pub fn draw(&mut self, vertices: core::ops::Range<u32>) {
        self.raw.draw(vertices, 0..1);
    }

    pub fn draw_indexed(&mut self, indices: core::ops::Range<u32>) {
        self.draw_indexed_instanced(indices, 0..1);
    }

    /// Draws `indices` once per instance in `instances`; per-instance vertex
    /// buffers advance over that range.
    pub fn draw_indexed_instanced(
        &mut self,
        indices: core::ops::Range<u32>,
        instances: core::ops::Range<u32>,
    ) {
        self.raw.draw_indexed(indices, 0, instances);
    }
}
