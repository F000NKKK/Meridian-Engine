//! GPU compute adapter for `physics-domain`'s mass-spring soft-body model
//! (`physics-domain::soft_body`) — the same role `meridian-gac-wgpu`
//! plays for GAC batch transforms, per
//! [dependency-rules.md rule 11](../../../docs/dependency-rules.md): a
//! new domain adopting GPU compute gets its own `meridian-<domain>-compute`
//! adapter crate (`physics-domain` + `compute-app`, plus `gpu-wgpu`
//! directly for the same resource-type-naming reason `gac-wgpu` needs
//! it — see that crate's own doc comment) rather than growing
//! `compute-app` itself or reaching into `compute-wgpu` directly.
//!
//! Layer: infrastructure (adapter); bounded context: physics
//! (`craftbook.toml`, docs/context-map.md).
//!
//! [`soft_body::csr`] builds the per-particle CSR spring adjacency
//! ([`soft_body::csr::build_adjacency`]) `soft_body::float`/
//! `soft_body::fixed` both dispatch from —
//! topology extraction has no float/fixed distinction of its own. The
//! GPU kernels themselves are *not* thin aliases over one generic
//! implementation, unlike most float/fixed splits in this workspace
//! (CLAUDE.md's "Float/Fixed branching" rule): this is a genuine
//! GPU-dispatch constraint, the same one that keeps `gac::Motor3`
//! concretely `f32` — [`soft_body::float`]'s WGSL kernel uses native `f32` GPU
//! arithmetic directly, while [`soft_body::fixed`]'s kernel emulates Q16.16 (via
//! `meridian_gac_wgpu::fixed_wgsl::FIXED_ARITHMETIC_LIB_WGSL`) since
//! WGSL has no fixed-point type; the two shader programs are structurally
//! different, not two copies of the same one.
//!
//! Design notes: docs/notes/meridian-physics-wgpu.md (`lib`).

#![forbid(unsafe_code)]

//! [`rigid_body`], [`broad_phase`], [`narrow_phase`] and
//! [`constraint_solver`] are a third, independent kind of work this
//! crate hosts: batching `physics-domain`'s *entire* rigid-body pipeline
//! (`Integrator`, `BroadPhase::find_candidate_pairs`,
//! `NarrowPhase::test_pair`/`generate_contacts`, `ConstraintSolver`)
//! through `compute-app` instead of a plain `for` loop — every step
//! docs/roadmap.md's "batching is additive later" note named is done.
//! All five are generic over `GaFlavor` rather than float/fixed-split,
//! since (like `physics-domain`'s own engine) none of them has a
//! GPU-dispatch constraint of its own — see each module's doc comment.
//! `NarrowPhase::generate_contacts` (unlike `test_pair`'s fixed 1:1
//! shape) produces a *variable* number of manifold points per pair;
//! [`narrow_phase::GenerateContactsKernel`] handles that with a
//! fixed-size-per-pair output slot capped at
//! [`narrow_phase::MAX_CONTACTS_PER_PAIR`] (matching
//! `physics-domain::collision::face_manifold`'s own point cap) rather than
//! a true dynamic prefix-sum — see that module's own doc comment.
//! [`constraint_solver`] is the hardest of the five (each contact
//! mutates two bodies, so naive parallelism races) — see that module's
//! own doc comment for the graph-coloring fix and why it reproduces the
//! sequential Gauss-Seidel solver's result exactly, not just
//! approximately.
//!
//! **[`broad_phase::float::BroadPhaseGpuKernel`] is the first of these five with a
//! real WGSL shader behind it** — the concrete start of turning "batched
//! through `compute-app`'s CPU backend" into "literally
//! GPU-executing." It only takes on the broad-phase overlap test itself
//! (six floats per body compared pairwise, pure arithmetic); each
//! body's [`meridian_physics_domain::dynamics::RigidBody::aabb`] is still
//! computed on the CPU before upload, since a `Cuboid` body's AABB needs
//! its rotor-transformed local axes — genuine GA work, same as the
//! integrator's rotor exponential/motor composition and narrow-phase's
//! SAT/manifold clipping, neither of which has a WGSL counterpart yet.
//! Porting those three faithfully (not just "compiles and looks
//! plausible") is real, disclosed follow-up work, not bundled in here.

mod batched_stepper;
pub mod broad_phase;
pub mod constraint_solver;
mod gpu_backend_missing;
pub mod narrow_phase;
mod readback;
pub mod rigid_body;
pub mod soft_body;

pub use batched_stepper::BatchedStepper;
pub use gpu_backend_missing::GpuBackendMissing;
