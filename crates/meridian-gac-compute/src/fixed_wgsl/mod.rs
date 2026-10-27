//! Q16.16 Fixed-point arithmetic in WGSL — GPU-side determinism, phase 1
//! of a larger plan (deterministic physics dispatched partly on GPU,
//! partly on CPU, with bit-identical results either way).
//!
//! `meridian_numeric_core::Fixed` is deliberately not `f32`/`f64`
//! precisely so its arithmetic is exactly specified (`i32`
//! `+`/`-`/`*`/`/`, not IEEE-754) — but WGSL has no native `i64`, and
//! `Fixed::mul`/`Fixed::div` both need a 64-bit intermediate to avoid
//! overflow (`Fixed::mul`: `(self.0 as i64 * rhs.0 as i64) >> 16`;
//! `Fixed::div`: `((self.0 as i64) << 16) / rhs.0 as i64`). This module
//! emulates that 64-bit arithmetic from 32-bit words — the standard
//! technique for a target with no native wide-integer type — and every
//! operation is validated bit-for-bit against the real CPU `Fixed` (not
//! just "close") via [`FixedArithmeticKernels::dispatch`]'s own test
//! suite below.
//!
//! Design notes: docs/notes/meridian-gac-compute.md (`fixed_wgsl`).

mod arithmetic_lib;
mod cordic_lib;
mod entry_points;
mod fixed_arithmetic_kernels;
mod fixed_binary_op;
#[cfg(test)]
mod tests;

pub use arithmetic_lib::FIXED_ARITHMETIC_LIB_WGSL;
pub use cordic_lib::CORDIC_LIB_WGSL;
pub(crate) use entry_points::dispatch_entry_points_wgsl;
pub use fixed_arithmetic_kernels::FixedArithmeticKernels;
pub use fixed_binary_op::FixedBinaryOp;
