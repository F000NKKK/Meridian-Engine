//! Compute dispatch runtime: device/context, buffers, and the `ComputeKernel` interface domain crates implement against. Knows nothing about physics, GAC, particles or any other algorithm — see docs/dependency-rules.md rule 5.
//!
//! Layer: application; bounded context: compute (`craftbook.toml`,
//! docs/context-map.md).
//!
//! [`ComputeContext::parallel_for`] is the one place the CPU backend gets
//! reached (rule 5): below `parallel_threshold` it runs sequentially on
//! the calling thread, at or above it delegates to the
//! [`ComputeDevice`] port (implemented by `compute-driver`). That threshold is
//! configurable state (via [`ComputeScheduler`]), not a hardcoded
//! constant domain crates read — the point being that "how many items
//! justify parallelizing" is a policy call, tunable per workload, not a
//! fact about the algorithm.
//!
//! `task-core` is a declared dependency (see docs/threading-model.md) but
//! not yet used here: `JobGraph::add_job` requires `'static` closures,
//! and `parallel_for`'s `work` is borrowed from the caller's stack frame
//! — a real signature mismatch, not an oversight. Reusing `task-core` for
//! this needs a scoped/borrowing job API that doesn't exist yet, so this
//! uses the device's own `std::thread::scope`-based dispatch
//! instead of forcing an awkward fit.
//!
//! Design notes: docs/notes/meridian-compute-app.md (`lib`).
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod backend_split;
mod compute_buffer;
mod compute_context;
mod compute_device;
mod compute_kernel;
mod compute_scheduler;
mod compute_support;
mod dispatch_size;
mod gpu_backend;
mod hybrid_kernel;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
#[cfg(test)]
mod tests;

pub use backend_split::BackendSplit;
pub use compute_buffer::ComputeBuffer;
pub use compute_context::ComputeContext;
pub use compute_device::ComputeDevice;
pub use compute_kernel::ComputeKernel;
pub use compute_scheduler::ComputeScheduler;
pub use compute_support::ComputeSupport;
pub use dispatch_size::DispatchSize;
pub use gpu_backend::GpuBackend;
pub use hybrid_kernel::HybridKernel;
