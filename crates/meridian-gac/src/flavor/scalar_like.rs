//! The scalar abstraction generic code is written against.

use core::ops::{Add, Div, Mul, Neg, Sub};

/// A scalar type usable throughout the generic (non-GPU-dispatch-bound)
/// half of this crate — implemented for [`meridian_numeric::Scalar`]
/// (`f32`) and [`meridian_numeric::Fixed`]. Every method is a thin forward to that type's own
/// inherent method (Rust resolves an inherent method over a
/// same-named trait method, so `self.sqrt()` inside `impl ScalarLike for
/// Scalar` calls `f32::sqrt`, not itself) — this trait exists to *name*
/// the operations generic code needs, not to reimplement them.
pub trait ScalarLike:
    Copy
    + core::fmt::Debug
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    const ZERO: Self;
    const ONE: Self;
    /// A small tolerance for degenerate (near-zero) guards — the same
    /// role `meridian_numeric::EPSILON` plays for `f32` and
    /// the fixed flavor's own internal tolerance plays for `Fixed`.
    const EPSILON: Self;
    /// The largest finite representable value — used as a "no candidate
    /// yet" sentinel by algorithms that scan for a minimum (e.g. SAT's
    /// least-overlap axis search), not for arithmetic.
    const MAX: Self;

    /// For authoring constants (`0.4` for a moment-of-inertia formula,
    /// `9.81` for gravity, ...) generically. Not meant for hot per-frame
    /// paths on the `Fixed` side (see `Fixed::from_num`'s own doc
    /// comment).
    fn from_f64(v: f64) -> Self;
    fn sqrt(self) -> Self;
    fn tan(self) -> Self;
    fn sin_cos(self) -> (Self, Self);
    fn signum(self) -> Self;
    fn abs(self) -> Self;
    fn min(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;
    fn clamp(self, lo: Self, hi: Self) -> Self;
}
