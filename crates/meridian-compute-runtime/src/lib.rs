//! Compute dispatch runtime: device/context, buffers, and the `ComputeKernel` interface domain crates implement against. Knows nothing about physics, GAC, particles or any other algorithm — see docs/dependency-rules.md rule 5.

/// Device-visible dispatch dimensions for a `ComputeKernel` invocation.
#[derive(Debug, Clone, Copy, Default)]
pub struct DispatchSize {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// Everything a `ComputeKernel` dispatch needs: bound buffers, the active
/// `compute-driver` backend, synchronization state. Consumers reach
/// `compute-driver` only through this type — see
/// docs/dependency-rules.md rule 5.
#[derive(Debug, Default)]
pub struct ComputeContext;

/// A dispatchable unit of compute work. Domain crates (`gac-compute`, and
/// future `particle-compute`/`physics-compute`/`ai-compute`) implement this
/// for their own kernel types; `compute-runtime` knows nothing about what a
/// kernel computes, only how it gets dispatched and on which backend.
pub trait ComputeKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize);
}

/// Orders and dispatches `ComputeKernel` submissions across whichever
/// `compute-driver` backend is active — command/queue-level ordering
/// within a compute submission, not general engine task scheduling (that's
/// `task-core`'s job; see docs/threading-model.md). CPU-vs-GPU dispatch
/// policy (e.g. a batch-size threshold) belongs here as configurable state
/// once this is implemented, not as a hardcoded value domain crates read.
#[derive(Debug, Default)]
pub struct ComputeScheduler;
