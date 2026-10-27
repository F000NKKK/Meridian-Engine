//! A kernel dispatched through a context.

use crate::ComputeContext;
use crate::DispatchSize;

/// A dispatchable unit of compute work. Domain crates (`gac-compute`, and
/// future `particle-compute`/`physics-compute`/`ai-compute`) implement this
/// for their own kernel types; `compute-runtime` knows nothing about what a
/// kernel computes, only how it gets dispatched and on which backend.
pub trait ComputeKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize);
}
