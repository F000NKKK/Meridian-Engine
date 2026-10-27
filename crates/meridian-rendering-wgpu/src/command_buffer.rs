//! Command recording.

use crate::{DepthTexture, RenderPass};

/// A recorded, submittable sequence of GPU commands. Wraps
/// [`meridian_gpu_wgpu::CommandBuffer`], adding
/// [`CommandBuffer::begin_render_pass`] — the graphics-specific recording
/// operation the shared crate deliberately doesn't know about.
#[derive(Debug)]
pub struct CommandBuffer<'a> {
    pub(crate) inner: meridian_gpu_wgpu::CommandBuffer<'a>,
}

impl<'a> CommandBuffer<'a> {
    /// Opens a render pass targeting `color_target` (typically a
    /// [`SurfaceFrame::view`](crate::SurfaceFrame::view)), cleared to `clear_color` (RGBA, `0.0..=1.0`)
    /// before anything is drawn, with an optional `depth` attachment
    /// (cleared to `1.0` — the far plane — matching
    /// [`Device::create_render_pipeline`](crate::Device::create_render_pipeline)'s `depth_enabled` +
    /// `CompareFunction::Less` convention: closer geometry has a *smaller*
    /// depth value). Recording ends when the returned [`RenderPass`](crate::RenderPass) is
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

    /// Opens a depth-only pass rendering into `shadow_map` (a
    /// [`Device::create_shadow_map_texture`](crate::Device::create_shadow_map_texture) result), cleared to `1.0`
    /// first — the shadow-casting geometry pass, recorded through
    /// [`Device::create_shadow_pipeline`](crate::Device::create_shadow_pipeline), before the main
    /// [`begin_render_pass`](Self::begin_render_pass) that samples the
    /// result.
    pub fn begin_shadow_pass<'pass>(
        &'pass mut self,
        shadow_map: &'pass meridian_gpu_wgpu::Texture,
    ) -> RenderPass<'pass> {
        let raw = self
            .inner
            .encoder_mut()
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: shadow_map.view(),
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
        self.inner.submit();
    }
}
