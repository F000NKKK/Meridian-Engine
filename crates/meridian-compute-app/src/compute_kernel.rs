//! A kernel dispatched through a context.

use crate::{ComputeContext, DispatchSize};

/// A dispatchable unit of compute work. Domain crates (`gac-wgpu`, and
/// future `particle-compute`/`physics-wgpu`/`ai-compute`) implement this
/// for their own kernel types; `compute-app` knows nothing about what a
/// kernel computes, only how it gets dispatched and on which backend.
pub trait ComputeKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize);
}
