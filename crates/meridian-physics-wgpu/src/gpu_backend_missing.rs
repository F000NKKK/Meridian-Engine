//! The error a GPU kernel returns when its `ComputeContext` has no GPU backend.

use meridian_compute_app::ComputeContext;
use meridian_compute_wgpu::GpuComputeDevice;

/// A GPU kernel was used with a [`ComputeContext`] that has no GPU backend
/// (see [`ComputeContext::with_gpu`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kernel} requires a ComputeContext with a GPU backend (see ComputeContext::with_gpu)")]
pub struct GpuBackendMissing {
    /// The kernel operation that needed the backend, for example `"BroadPhaseGpuKernel::new"`.
    pub kernel: &'static str,
}

/// The context's GPU device, or [`GpuBackendMissing`] naming `kernel`.
pub(crate) fn require_gpu<'a>(
    context: &'a ComputeContext,
    kernel: &'static str,
) -> Result<&'a GpuComputeDevice, GpuBackendMissing> {
    context
        .gpu::<GpuComputeDevice>()
        .ok_or(GpuBackendMissing { kernel })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use meridian_compute_app::testing::SequentialDevice;

    use super::*;

    #[test]
    fn context_without_gpu_is_reported_with_the_kernel_name() {
        let context = ComputeContext::new(Arc::new(SequentialDevice));
        let error = require_gpu(&context, "SomeKernel::new").unwrap_err();
        assert_eq!(
            error,
            GpuBackendMissing {
                kernel: "SomeKernel::new"
            }
        );
        assert!(error.to_string().starts_with("SomeKernel::new requires"));
    }
}
