//! GPU compute adapter for `physics-core`'s mass-spring soft-body model
//! (`physics-core::soft_body`) — the same role `meridian-gac-compute`
//! plays for GAC batch transforms, per
//! [dependency-rules.md rule 11](../../../docs/dependency-rules.md): a
//! new domain adopting GPU compute gets its own `meridian-<domain>-compute`
//! adapter crate (`physics-core` + `compute-runtime`, plus `gpu-driver`
//! directly for the same resource-type-naming reason `gac-compute` needs
//! it — see that crate's own doc comment) rather than growing
//! `compute-runtime` itself or reaching into `compute-driver` directly.
//!
//! [`generic`] builds the per-particle CSR spring adjacency
//! ([`generic::build_adjacency`]) `float`/`fixed` both dispatch from —
//! topology extraction has no float/fixed distinction of its own. The
//! GPU kernels themselves are *not* thin aliases over one generic
//! implementation, unlike most float/fixed splits in this workspace
//! (CLAUDE.md's "Float/Fixed branching" rule): this is a genuine
//! GPU-dispatch constraint, the same one that keeps `gac-core::Motor3`
//! concretely `f32` — [`float`]'s WGSL kernel uses native `f32` GPU
//! arithmetic directly, while [`fixed`]'s kernel emulates Q16.16 (via
//! `meridian_gac_compute::fixed_wgsl::FIXED_ARITHMETIC_LIB_WGSL`) since
//! WGSL has no fixed-point type; the two shader programs are structurally
//! different, not two copies of the same one.
//!
//! Both kernels mirror `SoftBodyIntegrator::step`'s *semantics* exactly
//! (same force-accumulation-then-integrate-then-collide structure, same
//! per-spring formulas — see each module's doc comment for the
//! particle-centric adjacency reformulation that makes this
//! race-condition-free without atomics) but only the `fixed` kernel
//! promises bit-exact reproducibility against the CPU path; `float`'s
//! GPU/CPU results agree numerically, not bit-for-bit (GPU float
//! summation order can differ from the CPU's, the same reason `Fixed`
//! exists at all — see `meridian_numeric_core::Fixed`'s doc comment).
//!
//! Like [`meridian_gac_compute::fixed_wgsl`], buffers (topology,
//! positions, velocities) are rebuilt every [`float::SoftBodyGpuKernel::step`]/
//! [`fixed::FixedSoftBodyGpuKernel::step`] call rather than persisted
//! across steps — a real, deliberately-deferred gap (see
//! `meridian_compute_runtime`'s `HybridKernel` module doc for the same
//! tradeoff acknowledged there), not an oversight.

//! [`rigid_body`], [`broad_phase`], [`narrow_phase`] and
//! [`constraint_solver`] are a third, independent kind of work this
//! crate hosts: batching `physics-core`'s *entire* rigid-body pipeline
//! (`Integrator`, `BroadPhase::find_candidate_pairs`,
//! `NarrowPhase::test_pair`/`generate_contacts`, `ConstraintSolver`)
//! through `compute-runtime` instead of a plain `for` loop — every step
//! docs/roadmap.md's "batching is additive later" note named is done.
//! All five are generic over `GaFlavor` rather than float/fixed-split,
//! since (like `physics-core`'s own engine) none of them has a
//! GPU-dispatch constraint of its own — see each module's doc comment.
//! `NarrowPhase::generate_contacts` (unlike `test_pair`'s fixed 1:1
//! shape) produces a *variable* number of manifold points per pair;
//! [`narrow_phase::GenerateContactsKernel`] handles that with a
//! fixed-size-per-pair output slot capped at
//! [`narrow_phase::MAX_CONTACTS_PER_PAIR`] (matching
//! `physics-core::generic::face_manifold`'s own point cap) rather than
//! a true dynamic prefix-sum — see that module's own doc comment.
//! [`constraint_solver`] is the hardest of the five (each contact
//! mutates two bodies, so naive parallelism races) — see that module's
//! own doc comment for the graph-coloring fix and why it reproduces the
//! sequential Gauss-Seidel solver's result exactly, not just
//! approximately.

pub mod broad_phase;
pub mod constraint_solver;
pub mod fixed;
pub mod float;
pub mod generic;
pub mod narrow_phase;
pub mod rigid_body;
