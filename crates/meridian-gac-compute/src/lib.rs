//! Batch execution kernels for `gac-core`'s `Motor3` transforms — the adapter between pure geometric algebra and `compute-runtime`'s dispatch interface. See docs/adr/007-batch-transforms-via-compute.md.
//!
//! Layer: infrastructure (adapter); bounded context: geometry
//! (`craftbook.toml`, docs/context-map.md).
//!
//! [`fixed_wgsl`] is a second, independent kind of GPU work this crate
//! hosts: `Fixed`-point (Q16.16) arithmetic running as real WGSL compute
//! shaders, bit-exact against the CPU `Fixed` implementation — phase 1 of
//! deterministic GPU physics (see that module's own doc comment for
//! scope and the `docs/roadmap.md` GPU-determinism entry for the larger
//! plan).
//!
//! `ComputeKernel::dispatch` takes `&self`, not `&mut self` (multiple
//! dispatch invocations can run concurrently against the same context),
//! so both kernels below hold their output in a `Mutex<Vec<Motor3>>`:
//! indexed writes under the lock, one lock acquisition per work item.
//! That's real per-item lock contention, not free — an acceptable first
//! version (correct, easy to verify by test) rather than the faster but
//! `unsafe`-requiring "each thread gets a disjoint `&mut` slice of the
//! output" version, matching this workspace's established policy of
//! safety-first with the optimization explicitly deferred (see
//! `meridian-task-core`'s shared-ready-queue-not-work-stealing-deques
//! note for the same tradeoff made elsewhere).

#![forbid(unsafe_code)]

//! [`FixedMotorTransformKernel`]/[`FixedMotorComposeKernel`] are the same
//! two kernels built on `gac-core::fixed_ga::FixedMotor3` instead —
//! deterministic, for `physics-core::fixed`-style consumers that need a
//! batch path too. CPU vs. GPU dispatch is a *setting* a caller picks,
//! not a type-level restriction this crate imposes (see
//! `gac-core`'s crate-root doc comment): `compute-runtime` has no GPU
//! backend implemented yet (see docs/roadmap.md), so both kernels here
//! only exercise the CPU dispatch path today. A future GPU backend could
//! in principle run `Fixed` kernels too, at the cost of `i64` emulation
//! and losing the bit-exact determinism guarantee to GPU execution-order
//! nondeterminism (see docs/adr/008-fixed-point-determinism.md) — a
//! tradeoff for that caller to accept knowingly when it configures a GPU
//! `compute-driver` backend, not something forbidden by these kernels.

mod fixed_motor_compose_kernel;
mod fixed_motor_transform_kernel;
pub mod fixed_wgsl;
mod motor_compose_kernel;
mod motor_transform_kernel;
#[cfg(test)]
mod tests;

pub use fixed_motor_compose_kernel::FixedMotorComposeKernel;
pub use fixed_motor_transform_kernel::FixedMotorTransformKernel;
pub use motor_compose_kernel::MotorComposeKernel;
pub use motor_transform_kernel::MotorTransformKernel;
