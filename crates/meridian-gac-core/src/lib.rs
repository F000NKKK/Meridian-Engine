//! Geometric Algebra Core — vectors, bivectors, multivectors, rotors and motors; the shared spatial math foundation for every other subsystem.
//!
//! Two flavors, in their own modules, each self-contained (algebra *and*
//! geometric primitives): [`float_ga`] (`f32`, the default,
//! GPU-dispatchable path — re-exported at the crate root, so
//! `meridian_gac_core::Vec3`/`Motor3`/`Aabb`/etc. resolve to it unchanged)
//! and [`fixed_ga`] (`Fixed`, deterministic, opt-in — usable by *any*
//! crate that needs CPU-deterministic geometry, not just
//! `physics-core`'s `DeterministicBody`: this is where that reusability
//! has to live, the same reason `float_ga`'s primitives aren't
//! `physics-core`-local either). See [`fixed_ga`]'s module doc comment
//! for why both exist instead of one generic implementation. See
//! docs/gac-design.md and
//! [ADR 001](../../../docs/adr/001-geometric-algebra-as-spatial-model.md).

pub mod fixed_ga;
pub mod float_ga;

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
