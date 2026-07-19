//! Shared compute scheduling and buffer management for subsystems that need CPU SIMD / GPU compute: physics, rendering, and future animation/particles/AI cores.

/// A scheduled compute workload, e.g. a physics broad-phase pass or a GPU
/// culling pass. Consumers reach `compute-driver` only through this type —
/// see docs/dependency-rules.md rule 5.
#[derive(Debug, Clone, Default)]
pub struct ComputeTask {
    pub name: &'static str,
}

/// Below this many elements, `ComputeScheduler` prefers running a task on
/// the CPU path; at or above it, GPU compute dispatch through
/// `compute-driver` wins on throughput despite upload/sync latency. Generic
/// to any batched workload — domain crates (e.g. `meridian-gac-compute`)
/// reuse this rather than defining their own crossover point.
pub const GPU_DISPATCH_THRESHOLD: usize = 10_000;

/// Schedules `ComputeTask`s across whichever `compute-driver` backend is
/// active, picking CPU or GPU per task based on `GPU_DISPATCH_THRESHOLD`.
#[derive(Debug, Default)]
pub struct ComputeScheduler {
    pub pending: Vec<ComputeTask>,
}
