//! Command recording and submission.

use crate::{BindGroup, Buffer, ComputePipeline, Device};

/// A recorded, submittable sequence of GPU commands. Borrows the
/// [`Device`](crate::Device) it was opened from ([`Device::create_command_buffer`](crate::Device::create_command_buffer)) so a
/// `CommandBuffer` can't outlive its device; nothing is submitted to the
/// GPU until [`CommandBuffer::submit`] consumes it.
#[derive(Debug)]
pub struct CommandBuffer<'a> {
    pub(crate) device: &'a Device,
    pub(crate) encoder: wgpu::CommandEncoder,
}

impl CommandBuffer<'_> {
    /// Records a compute dispatch: binds `buffer` at `@group(0)
    /// @binding(0)` (the only binding shape supported so far — see
    /// [`Device::create_compute_pipeline`](crate::Device::create_compute_pipeline)'s doc comment) and dispatches
    /// `workgroups` workgroups along X (Y/Z left at 1 — 1D dispatch is
    /// all today's callers need; extending to 3D is additive).
    pub fn dispatch_compute(
        &mut self,
        pipeline: &ComputePipeline,
        buffer: &Buffer,
        workgroups: u32,
    ) {
        let bind_group_layout = pipeline.raw.get_bind_group_layout(0);
        let bind_group = self
            .device
            .create_single_buffer_bind_group(&bind_group_layout, buffer);

        let mut pass = self
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline.raw);
        pass.set_bind_group(0, &bind_group.raw, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Records a compute dispatch against an already-built `bind_group`
    /// (see [`Device::create_bind_group`](crate::Device::create_bind_group)) — the general form of
    /// [`CommandBuffer::dispatch_compute`] for pipelines that need more
    /// than the single-buffer binding shape it assumes.
    pub fn dispatch_compute_with_bind_group(
        &mut self,
        pipeline: &ComputePipeline,
        bind_group: &BindGroup,
        workgroups: u32,
    ) {
        let mut pass = self
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline.raw);
        pass.set_bind_group(0, &bind_group.raw, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Escape hatch for `rendering-wgpu`'s render-pass recording, which
    /// this crate deliberately doesn't know about (render
    /// passes/swapchains are a graphics-specific concept — see the module
    /// doc). Prefer [`CommandBuffer::dispatch_compute`] where it covers
    /// the need.
    pub fn encoder_mut(&mut self) -> &mut wgpu::CommandEncoder {
        &mut self.encoder
    }

    /// The [`Device`](crate::Device) this buffer was opened from — needed by
    /// `rendering-wgpu`'s render-pass recording alongside
    /// [`CommandBuffer::encoder_mut`].
    #[must_use]
    pub fn device(&self) -> &Device {
        self.device
    }

    /// Submits every recorded command to the device's queue.
    pub fn submit(self) {
        self.device.queue.submit(Some(self.encoder.finish()));
    }
}
