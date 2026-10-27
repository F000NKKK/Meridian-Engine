//! Low-level compute dispatch abstraction (CPU SIMD backends, GPU compute queues/buffers). Knows nothing about physics, animation or rendering.
//!
//! Layer: infrastructure (adapter); bounded context: compute
//! (`craftbook.toml`, docs/context-map.md).
//!
//! The adapter for `compute-app`'s ports (ADR 019). Two independent
//! backends. [`CpuComputeDevice`] implements `ComputeDevice` and runs work
//! across real OS threads via `std::thread::scope`, safe, no `unsafe`.
//! [`GpuComputeDevice`] implements `GpuBackend` and is
//! real too now, built on [`meridian_gpu_wgpu`] — the crate shared with
//! `rendering-wgpu` that owns the actual `wgpu` device/buffer/shader
//! mechanics (see that crate's module doc and
//! [ADR 011](../../../docs/adr/011-shared-gpu-driver-crate.md)); this
//! crate adds nothing on top but the compute-dispatch-shaped API
//! (`allocate_buffer`/`dispatch`) kernel adapters use. The composition
//! root builds the devices and hands them to a `ComputeContext`
//! (`ComputeContext::new` for CPU-only, `with_gpu` to add the GPU path
//! alongside it) — there is no automatic fallback between them.
//!
//! Design notes: docs/notes/meridian-compute-wgpu.md (`lib`).

#![forbid(unsafe_code)]

mod compute_capabilities;
mod contexts;
mod cpu_compute_device;
mod generation_cache;
mod gpu_compute_device;
mod scratch_pool;
mod scratch_slot;
#[cfg(test)]
mod tests;

pub use compute_capabilities::ComputeCapabilities;
pub use contexts::{cpu_context, gpu_context};
pub use cpu_compute_device::CpuComputeDevice;
pub use generation_cache::GenerationCache;
pub use gpu_compute_device::{GpuComputeDevice, GpuComputeError};
pub use scratch_pool::ScratchPool;
pub use scratch_slot::ScratchSlot;
