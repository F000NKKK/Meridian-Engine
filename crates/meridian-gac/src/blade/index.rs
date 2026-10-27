//! Named blade indices into a `Multivector`'s 16 components.

/// Named blade indices into a `Multivector`'s 16 components, encoded as a
/// 4-bit mask over `{e0, e1, e2, e3}` (bit i set means `ei` is a factor).
/// Blades are always stored/read in canonical increasing-index order.
///
/// Pure integer bitmask constants — not tied to `Scalar` vs `Fixed` at
/// all — so they live once here, shared by both
/// [`crate::Multivector`] and [`crate::FixedMultivector`], rather than
/// being copied into each.
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
