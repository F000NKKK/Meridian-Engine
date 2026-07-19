//! Shared compute scheduling and buffer management for subsystems that need CPU SIMD / GPU compute: physics, rendering, and future animation/particles/AI cores.

use meridian_gac_core::Motor3;

/// A scheduled compute workload, e.g. a physics broad-phase pass or a GPU
/// culling pass. Consumers reach `compute-driver` only through this type —
/// see docs/dependency-rules.md rule 5.
#[derive(Debug, Clone, Default)]
pub struct ComputeTask {
    pub name: &'static str,
}

/// Below this many elements, `ComputeScheduler` prefers running a batch
/// kernel on the CPU path; at or above it, GPU compute dispatch through
/// `compute-driver` wins on throughput despite upload/sync latency. See
/// docs/gac-design.md "Batch execution via compute-core".
pub const GPU_DISPATCH_THRESHOLD: usize = 10_000;

/// Schedules `ComputeTask`s across whichever `compute-driver` backend is
/// active, picking CPU or GPU per task based on `GPU_DISPATCH_THRESHOLD`.
#[derive(Debug, Default)]
pub struct ComputeScheduler {
    pub pending: Vec<ComputeTask>,
}

/// Batch-transforms local-space `Motor3`s into world space as a single
/// `ComputeTask`. This is the shared entry point `ecs-core`'s transform
/// propagation, `physics-core`, and `graphics-core` use to move large
/// batches of transforms onto CPU-SIMD or GPU compute — `gac-core` itself
/// stays backend-agnostic and never depends on `compute-core` (rule 6);
/// this type is what lets the same `Motor3` math run scalar-CPU for small
/// counts and GPU-batched for large ones without gac-core knowing either
/// way. See docs/gac-design.md.
#[derive(Debug, Clone, Default)]
pub struct TransformBatchKernel {
    pub task: ComputeTask,
    pub locals: Vec<Motor3>,
}
