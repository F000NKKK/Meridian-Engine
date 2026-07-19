//! Batch execution kernels for `gac-core`'s `Motor3` transforms — the adapter between pure geometric algebra and `compute-core`'s CPU-SIMD/GPU runtime. See docs/adr/007-batch-transforms-via-compute.md.

use meridian_compute_core::{ComputeTask, GPU_DISPATCH_THRESHOLD};
use meridian_gac_core::Motor3;

/// Batch-transforms local-space `Motor3`s into world space as a single
/// `ComputeTask`, dispatched via `compute-core` on CPU-SIMD below
/// `GPU_DISPATCH_THRESHOLD` elements or GPU compute at or above it. This is
/// the entry point `ecs-core`'s transform propagation, `physics-core`, and
/// `graphics-core` use for large batches; single-transform math (`motor *
/// local`) stays direct `gac-core` calls and never goes through this crate.
#[derive(Debug, Clone, Default)]
pub struct MotorTransformKernel {
    pub task: ComputeTask,
    pub locals: Vec<Motor3>,
}

impl MotorTransformKernel {
    /// Whether `execute` would prefer the GPU compute path for this batch,
    /// per `compute-core`'s shared CPU/GPU crossover point.
    pub fn prefers_gpu(&self) -> bool {
        self.locals.len() >= GPU_DISPATCH_THRESHOLD
    }
}
