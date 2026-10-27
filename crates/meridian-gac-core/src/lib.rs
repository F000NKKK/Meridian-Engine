//! Geometric Algebra Core — vectors, bivectors, multivectors, rotors and motors; the shared spatial math foundation for every other subsystem.
//!
//! Layer: shared kernel; bounded context: geometry (`craftbook.toml`,
//! docs/context-map.md). A performance kernel (Craftbook §7.8): its
//! data-oriented layout is provisional until a benchmark justifies it
//! (ADR 020 D8).
//!
//! Two flavors, in their own modules: [`float_ga`] (`f32`, the default —
//! re-exported at the crate root, so `meridian_gac_core::Vec3`/`Motor3`/
//! `Aabb`/etc. resolve to it unchanged) and [`fixed_ga`] (`Fixed`,
//! deterministic — usable by *any* crate that needs CPU-deterministic
//! geometry, not just `physics-core`). Only `Multivector`/`Vec3`/
//! `Bivector3`/`Rotor`/`Motor3` themselves are concretely duplicated
//! between the two (see [`fixed_ga`]'s module doc comment for why:
//! `meridian-gac-compute` dispatches them, and `compute-runtime` has no
//! GPU backend to dispatch to at all yet — CPU vs. GPU is a dispatch
//! *setting* a caller picks once one exists, not a type-level
//! restriction this crate imposes; a future GPU backend could in
//! principle run `Fixed` kernels too, at the cost of `i64` emulation and
//! losing the bit-exact determinism guarantee to GPU execution-order
//! nondeterminism — a tradeoff for the caller to accept knowingly, not
//! something forbidden here).
//!
//! Design notes: docs/notes/meridian-gac-core.md (`lib`).

#![forbid(unsafe_code)]

#[cfg(feature = "data")]
mod data;
pub mod fixed_ga;
pub mod float_ga;
pub mod generic;

#[cfg(feature = "data")]
pub use data::MotorForm;
pub use float_ga::*;

/// Named blade indices into a `Multivector`'s 16 components, encoded as a
/// 4-bit mask over `{e0, e1, e2, e3}` (bit i set means `ei` is a factor).
/// Blades are always stored/read in canonical increasing-index order.
///
/// Pure integer bitmask constants — not tied to `Scalar` vs `Fixed` at
/// all — so this lives once here, shared by both
/// [`float_ga::Multivector`] and [`fixed_ga::FixedMultivector`], rather
/// than being copied into each.
pub mod blade {
    pub const S: usize = 0b0000;
    pub const E0: usize = 0b0001;
    pub const E1: usize = 0b0010;
    pub const E2: usize = 0b0100;
    pub const E3: usize = 0b1000;
    pub const E01: usize = 0b0011;
    pub const E02: usize = 0b0101;
    pub const E03: usize = 0b1001;
    pub const E12: usize = 0b0110;
    pub const E13: usize = 0b1010;
    pub const E23: usize = 0b1100;
    pub const E012: usize = 0b0111;
    pub const E013: usize = 0b1011;
    pub const E023: usize = 0b1101;
    pub const E123: usize = 0b1110;
    pub const E0123: usize = 0b1111;
}
