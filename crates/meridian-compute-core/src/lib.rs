//! Shared compute scheduling and buffer management for subsystems that need CPU SIMD / GPU compute: physics, rendering, and future animation/particles/AI cores.

/// A scheduled compute workload, e.g. a physics broad-phase pass or a GPU
/// culling pass. Consumers reach `compute-driver` only through this type —
/// see docs/dependency-rules.md rule 5.
#[derive(Debug, Clone, Default)]
pub struct ComputeTask {
    pub name: &'static str,
}

/// Schedules `ComputeTask`s across whichever `compute-driver` backend is active.
#[derive(Debug, Default)]
pub struct ComputeScheduler {
    pub pending: Vec<ComputeTask>,
}
