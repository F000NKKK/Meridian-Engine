//! A compiled render pipeline.

/// A configured render pipeline — vertex + fragment stages, primitive/
/// culling state, and an optional depth-test stage. Built by
/// [`Device::create_render_pipeline`](crate::Device::create_render_pipeline), bound via
/// [`RenderPass::set_pipeline`](crate::RenderPass::set_pipeline).
#[derive(Debug)]
pub struct RenderPipeline {
    pub(crate) raw: wgpu::RenderPipeline,
}
