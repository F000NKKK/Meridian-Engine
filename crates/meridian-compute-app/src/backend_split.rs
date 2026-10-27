//! How a hybrid dispatch divides items between the CPU and the GPU.

/// How [`ComputeContext::run_hybrid`](crate::ComputeContext::run_hybrid) splits `count` work items between
/// CPU and GPU.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackendSplit {
    /// Every item on the CPU.
    CpuOnly,
    /// Every item on the GPU.
    GpuOnly,
    /// `0.0..=1.0`: that fraction of items on the GPU, the rest on the
    /// CPU. `Ratio(0.5)` is an even 50/50 split. Out-of-range values are
    /// clamped, not a logic error — a caller computing a ratio
    /// dynamically (e.g. from a benchmark) shouldn't have to re-clamp it
    /// themselves.
    Ratio(f32),
}

impl BackendSplit {
    pub(crate) fn gpu_item_count(self, count: usize, gpu_available: bool) -> usize {
        if !gpu_available {
            return 0;
        }
        match self {
            BackendSplit::CpuOnly => 0,
            BackendSplit::GpuOnly => count,
            BackendSplit::Ratio(fraction) => {
                let fraction = fraction.clamp(0.0, 1.0);
                ((count as f32) * fraction).round() as usize
            }
        }
    }
}
