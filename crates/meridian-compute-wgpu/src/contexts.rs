//! The composition root's one-liners for the default compute contexts.

use std::sync::Arc;

use meridian_compute_app::ComputeContext;

use crate::{CpuComputeDevice, GpuComputeDevice, GpuComputeError};

/// A CPU-only [`ComputeContext`] over a [`CpuComputeDevice`]: the composition
/// root's one-liner for the default backend.
#[must_use]
pub fn cpu_context() -> ComputeContext {
    ComputeContext::new(Arc::new(CpuComputeDevice::new()))
}

/// [`cpu_context`] plus a GPU backend. A real `async fn` (acquiring the device
/// is genuine I/O).
///
/// # Errors
///
/// [`GpuComputeError`] when there is no adapter: there is no silent fallback
/// to CPU-only, so a caller that wants the GPU finds out here.
pub async fn gpu_context() -> Result<ComputeContext, GpuComputeError> {
    Ok(cpu_context().with_gpu(Arc::new(GpuComputeDevice::new().await?)))
}
