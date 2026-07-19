//! [`Fixed`]: a deterministic alternative to [`crate::float::Scalar`] for
//! simulation state that must reproduce bit-identically across platforms
//! (lockstep networking, replay). See the type's own doc comment for why.

use core::ops::{Add, Div, Mul, Neg, Sub};

/// Q16.16 fixed-point number: 16 integer bits (range roughly ±32768), 16
/// fractional bits (resolution `1/65536` ≈ `1.5e-5`), backed by `i32`.
///
/// This exists for **deterministic simulation** — the same sequence of
/// operations on the same inputs must produce bit-identical results on
/// every platform/build this runs on, which plain `f32`/`f64` cannot
/// promise: IEEE-754 leaves rounding-mode, FMA fusion, and
/// extended-precision-register behavior implementation-defined, and
/// different compilers/CPUs/optimization levels really do disagree on the
/// last bit or two in practice. Fixed-point integer arithmetic has none of
/// that: `+`/`-`/`*`/`/` on `i32`/`i64` are exactly specified, so `Fixed`
/// is exactly reproducible. The cost is precision (fixed ~1.5e-5
/// resolution everywhere, vs. `f32`'s relative precision) and range
/// (±32768) — acceptable for the physics-scale quantities this workspace
/// deals with, not a general-purpose float replacement.
///
/// `sqrt` is exact (Newton's method on the underlying integer, not a
/// polynomial approximation). `sin`/`cos`/`atan2` use
/// [CORDIC](https://en.wikipedia.org/wiki/CORDIC) — the standard way to
/// compute trig functions from only add/subtract/shift, which is what
/// makes them exactly reproducible too; a polynomial or lookup-table
/// approximation calling into `libm` would reintroduce the same
/// platform-dependence `Fixed` exists to remove.
///
/// This is opt-in, not a replacement for [`crate::float::Scalar`]:
/// `gac-core` exposes both a `float_ga` (the everyday, GPU-dispatchable
/// path — see `meridian-gac-compute`) and a `fixed_ga` module (this
/// type's geometric algebra, used by `physics-core`'s deterministic
/// simulation path only) built on top of it. See
/// `physics-core::DeterministicBody` and docs/roadmap.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Fixed(i32);

impl Fixed {
    const FRAC_BITS: u32 = 16;

    pub const ZERO: Fixed = Fixed(0);
    pub const ONE: Fixed = Fixed(1 << Self::FRAC_BITS);

    /// Constructs a `Fixed` directly from its raw Q16.16 bit pattern.
    pub const fn from_bits(bits: i32) -> Self {
        Fixed(bits)
    }

    /// The raw Q16.16 bit pattern.
    pub const fn to_bits(self) -> i32 {
        self.0
    }

    /// Converts from an `f64` (e.g. a literal constant like `9.81`) by
    /// rounding to the nearest representable Q16.16 value. Not `const` (no
    /// stable const float rounding), and not meant for hot per-frame
    /// paths — for authoring constants and test oracles.
    pub fn from_num(value: f64) -> Self {
        Fixed((value * (1i64 << Self::FRAC_BITS) as f64).round() as i32)
    }

    /// Converts to an `f64` — for debug printing and comparing against a
    /// floating-point oracle in tests, not for simulation-critical paths
    /// (that would reintroduce the platform-dependence `Fixed` exists to
    /// avoid).
    pub fn to_num(self) -> f64 {
        self.0 as f64 / (1i64 << Self::FRAC_BITS) as f64
    }

    pub fn pi() -> Fixed {
        Fixed::from_num(core::f64::consts::PI)
    }

    pub fn half_pi() -> Fixed {
        Fixed::from_num(core::f64::consts::FRAC_PI_2)
    }

    pub fn two_pi() -> Fixed {
        Fixed::from_num(core::f64::consts::TAU)
    }

    pub fn abs(self) -> Self {
        Fixed(self.0.abs())
    }

    pub fn signum(self) -> Self {
        match self.0.cmp(&0) {
            core::cmp::Ordering::Greater => Fixed::ONE,
            core::cmp::Ordering::Less => -Fixed::ONE,
            core::cmp::Ordering::Equal => Fixed::ZERO,
        }
    }

    pub fn min(self, other: Self) -> Self {
        if self.0 < other.0 { self } else { other }
    }

    pub fn max(self, other: Self) -> Self {
        if self.0 > other.0 { self } else { other }
    }

    pub fn clamp(self, lo: Self, hi: Self) -> Self {
        self.max(lo).min(hi)
    }

    /// Exact integer square root of the fixed value (floor-rounded to the
    /// nearest representable Q16.16 step), via Newton's method on the
    /// underlying integer — not a polynomial approximation.
    ///
    /// # Panics
    /// Panics if `self` is negative — every call site in this workspace
    /// takes a length-squared or similarly non-negative quantity, so a
    /// negative input is a logic error to surface immediately, the same
    /// way `f32::sqrt` returning `NaN` for a negative input is usually a
    /// silently-propagating bug rather than a useful value.
    pub fn sqrt(self) -> Self {
        assert!(self.0 >= 0, "Fixed::sqrt of a negative value");
        let scaled = (self.0 as u64) << Self::FRAC_BITS;
        Fixed(isqrt_u64(scaled) as i32)
    }

    /// `(sin(self), cos(self))` via CORDIC rotation mode. See the type's
    /// doc comment for why CORDIC, not a `libm` call or a polynomial fit.
    pub fn sin_cos(self) -> (Fixed, Fixed) {
        let reduced = reduce_to_pi_range(self);
        let half_pi = Fixed::half_pi();
        let pi = Fixed::pi();

        let (theta, negate_cos) = if reduced > half_pi {
            (pi - reduced, true)
        } else if reduced < -half_pi {
            (-pi - reduced, true)
        } else {
            (reduced, false)
        };

        let (sin_theta, cos_theta) = cordic_rotate(theta);
        if negate_cos {
            (sin_theta, -cos_theta)
        } else {
            (sin_theta, cos_theta)
        }
    }

    pub fn sin(self) -> Fixed {
        self.sin_cos().0
    }

    pub fn cos(self) -> Fixed {
        self.sin_cos().1
    }

    /// # Panics
    /// Panics if `cos(self)` rounds exactly to zero (the angle is exactly
    /// a multiple of pi/2) — `Fixed` has no representation for infinity.
    pub fn tan(self) -> Fixed {
        let (sin, cos) = self.sin_cos();
        sin / cos
    }

    /// `atan2(self, x)` (`self` is the y-coordinate) via CORDIC vectoring
    /// mode. See the type's doc comment for why CORDIC.
    pub fn atan2(self, x: Fixed) -> Fixed {
        if x.0 == 0 && self.0 == 0 {
            return Fixed::ZERO;
        }
        if x.0 < 0 {
            // Reflect into the right half-plane (atan2's vectoring-mode
            // derivation only converges for x >= 0), then correct by ±pi:
            // atan2(y, x) = atan2(-y, -x) + pi (y >= 0) or - pi (y < 0).
            let base = cordic_vector_angle(Fixed(-self.0), Fixed(-x.0));
            return if self.0 >= 0 {
                base + Fixed::pi()
            } else {
                base - Fixed::pi()
            };
        }
        cordic_vector_angle(self, x)
    }
}

impl Add for Fixed {
    type Output = Fixed;
    fn add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0 + rhs.0)
    }
}

impl Sub for Fixed {
    type Output = Fixed;
    fn sub(self, rhs: Fixed) -> Fixed {
        Fixed(self.0 - rhs.0)
    }
}

impl Neg for Fixed {
    type Output = Fixed;
    fn neg(self) -> Fixed {
        Fixed(-self.0)
    }
}

impl Mul for Fixed {
    type Output = Fixed;
    fn mul(self, rhs: Fixed) -> Fixed {
        let product = (self.0 as i64 * rhs.0 as i64) >> Fixed::FRAC_BITS;
        Fixed(product as i32)
    }
}

impl Div for Fixed {
    type Output = Fixed;
    /// # Panics
    /// Panics if `rhs` is zero (integer division by zero) — `Fixed` has
    /// no representation for infinity/NaN.
    fn div(self, rhs: Fixed) -> Fixed {
        let numerator = (self.0 as i64) << Fixed::FRAC_BITS;
        Fixed((numerator / rhs.0 as i64) as i32)
    }
}

/// Floor integer square root via Newton's method (Heron's method): starts
/// from `n` itself and iterates `x -> (x + n/x) / 2` until it stops
/// decreasing, which converges to `floor(sqrt(n))` exactly for any `n`.
fn isqrt_u64(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Iteration count for the CORDIC loops below: each iteration adds
/// roughly one bit of precision, and Q16.16 has 16 fractional bits, so
/// this is already well past the point of any additional iteration
/// changing the rounded Q16.16 result.
const CORDIC_ITERATIONS: usize = 24;

/// The classic CORDIC gain constant, `prod_{i=0}^{inf} 1/sqrt(1+2^-2i)` —
/// rotation-mode CORDIC scales the vector's length by this factor, so
/// seeding `x` with it up front makes the final `(x, y)` already correctly
/// scaled instead of needing a separate correction pass.
const CORDIC_GAIN: f64 = 0.607_252_935_008_881_2;

/// `atan(2^-i)` for `i` in `0..CORDIC_ITERATIONS`, in radians. The
/// standard, widely-published CORDIC angle table for `i <= 15`; for
/// `i >= 16`, `atan(x) - x` is smaller than `2^-48` for `x = 2^-16`, far
/// below Q16.16's own `2^-16` resolution, so `2^-i` itself is used
/// directly rather than restating the same value to more digits than
/// matter.
fn cordic_atan_table() -> [Fixed; CORDIC_ITERATIONS] {
    const KNOWN: [f64; 16] = [
        core::f64::consts::FRAC_PI_4, // atan(1)
        0.463_647_609_000_806_1,
        0.244_978_663_126_864_14,
        0.124_354_994_546_761_44,
        0.062_418_809_995_957_35,
        0.031_239_833_430_268_277,
        0.015_623_728_620_476_831,
        0.007_812_341_060_101_111,
        0.003_906_230_131_966_972,
        0.001_953_122_516_478_818_8,
        0.000_976_562_189_559_319_5,
        0.000_488_281_211_194_898_3,
        0.000_244_140_620_149_361_78,
        0.000_122_070_311_893_670_21,
        0.000_061_035_156_174_208_77,
        0.000_030_517_578_115_526_096,
    ];
    let mut table = [Fixed::ZERO; CORDIC_ITERATIONS];
    for (i, slot) in table.iter_mut().enumerate() {
        *slot = if i < KNOWN.len() {
            Fixed::from_num(KNOWN[i])
        } else {
            Fixed::from_num(1.0 / (1u64 << i) as f64)
        };
    }
    table
}

/// Reduces `angle` into `(-pi, pi]` via `rem_euclid`, exact and
/// deterministic for any `i32` input (no repeated subtraction loop that
/// would be slow for large angles).
fn reduce_to_pi_range(angle: Fixed) -> Fixed {
    let two_pi_bits = Fixed::two_pi().0;
    let pi_bits = Fixed::pi().0;
    let reduced = angle.0.rem_euclid(two_pi_bits);
    Fixed(if reduced > pi_bits {
        reduced - two_pi_bits
    } else {
        reduced
    })
}

/// CORDIC rotation mode: rotates the vector `(CORDIC_GAIN, 0)` by `theta`
/// (which must be in `[-pi/2, pi/2]`) using only shift/add, converging to
/// `(cos(theta), sin(theta))`. Returns `(sin, cos)`.
fn cordic_rotate(theta: Fixed) -> (Fixed, Fixed) {
    let table = cordic_atan_table();
    let mut x = Fixed::from_num(CORDIC_GAIN);
    let mut y = Fixed::ZERO;
    let mut z = theta;
    for (i, &angle) in table.iter().enumerate() {
        let x_shifted = Fixed(x.0 >> i);
        let y_shifted = Fixed(y.0 >> i);
        if z.0 >= 0 {
            (x, y) = (x - y_shifted, y + x_shifted);
            z = z - angle;
        } else {
            (x, y) = (x + y_shifted, y - x_shifted);
            z = z + angle;
        }
    }
    (y, x)
}

/// CORDIC vectoring mode for `x >= 0`: rotates `(x, y)` toward the x-axis,
/// accumulating the rotation angle, which converges to `atan2(y, x)`.
fn cordic_vector_angle(y: Fixed, x: Fixed) -> Fixed {
    let table = cordic_atan_table();
    let mut cx = x;
    let mut cy = y;
    let mut z = Fixed::ZERO;
    for (i, &angle) in table.iter().enumerate() {
        let x_shifted = Fixed(cx.0 >> i);
        let y_shifted = Fixed(cy.0 >> i);
        if cy.0 >= 0 {
            (cx, cy) = (cx + y_shifted, cy - x_shifted);
            z = z + angle;
        } else {
            (cx, cy) = (cx - y_shifted, cy + x_shifted);
            z = z - angle;
        }
    }
    z
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_fixed_approx(got: Fixed, want: f64, tolerance: f64) {
        let diff = (got.to_num() - want).abs();
        assert!(
            diff <= tolerance,
            "expected ~{want}, got {} (diff {diff}, tolerance {tolerance})",
            got.to_num()
        );
    }

    // Q16.16's own resolution floor.
    const EPS: f64 = 1.0 / 65536.0;

    #[test]
    fn from_num_to_num_round_trips_within_resolution() {
        for v in [0.0, 1.0, -1.0, 3.5, -3.5, 100.25, -0.001, 12345.6789] {
            assert_fixed_approx(Fixed::from_num(v), v, EPS);
        }
    }

    #[test]
    fn arithmetic_matches_f64_oracle() {
        let cases: &[(f64, f64)] = &[
            (2.5, 4.0),
            (-3.0, 7.25),
            (0.0, 5.0),
            (123.5, -0.5),
            (-2.0, -2.0),
        ];
        for &(a, b) in cases {
            let (fa, fb) = (Fixed::from_num(a), Fixed::from_num(b));
            assert_fixed_approx(fa + fb, a + b, EPS);
            assert_fixed_approx(fa - fb, a - b, EPS);
            assert_fixed_approx(fa * fb, a * b, EPS * (a.abs() + b.abs() + 1.0));
            if b != 0.0 {
                assert_fixed_approx(fa / fb, a / b, EPS * (a.abs() + 1.0));
            }
        }
    }

    #[test]
    fn ordering_is_exact_and_total() {
        assert!(Fixed::from_num(1.0) < Fixed::from_num(2.0));
        assert!(Fixed::from_num(-1.0) < Fixed::from_num(0.0));
        assert_eq!(Fixed::from_num(1.0), Fixed::from_num(1.0));
    }

    #[test]
    fn sqrt_matches_f64_oracle() {
        for v in [0.0, 1.0, 2.0, 4.0, 9.0, 100.0, 0.25, 12345.6789] {
            assert_fixed_approx(Fixed::from_num(v).sqrt(), v.sqrt(), 4.0 * EPS);
        }
    }

    #[test]
    #[should_panic(expected = "negative")]
    fn sqrt_of_negative_panics() {
        Fixed::from_num(-1.0).sqrt();
    }

    #[test]
    fn sin_cos_matches_f64_oracle_across_the_full_circle() {
        // A tolerance several times Q16.16's own resolution floor:
        // CORDIC's fixed-point rounding at each of 24 iterations, plus
        // the reflection branch's own subtraction, accumulates a little
        // more error than a single rounding — still tiny in absolute
        // terms (well under 1/2000), far tighter than anything a real
        // simulation would notice.
        let tolerance = 32.0 * EPS;
        let mut degrees = -720i32;
        while degrees <= 720 {
            let radians = degrees as f64 * core::f64::consts::PI / 180.0;
            let (sin, cos) = Fixed::from_num(radians).sin_cos();
            assert_fixed_approx(sin, radians.sin(), tolerance);
            assert_fixed_approx(cos, radians.cos(), tolerance);
            degrees += 15;
        }
    }

    #[test]
    fn tan_matches_f64_oracle_away_from_the_asymptotes() {
        // Relative, not absolute, tolerance: tan's magnitude varies
        // hugely across this range (from 0 to ~5.7 near 80 degrees), and
        // dividing sin/cos amplifies their own small absolute error in
        // proportion to tan's magnitude — a single absolute bound would
        // either be too loose near zero or too tight near the asymptotes.
        for degrees in [-80, -45, -30, 0, 30, 45, 60, 80] {
            let radians = degrees as f64 * core::f64::consts::PI / 180.0;
            let want = radians.tan();
            let got = Fixed::from_num(radians).tan().to_num();
            let relative_tolerance = 0.002 * want.abs().max(1.0);
            assert!(
                (got - want).abs() <= relative_tolerance,
                "at {degrees} degrees: expected ~{want}, got {got}"
            );
        }
    }

    #[test]
    fn atan2_matches_f64_oracle_in_every_quadrant() {
        let tolerance = 8.0 * EPS;
        let cases: &[(f64, f64)] = &[
            (1.0, 1.0),
            (1.0, -1.0),
            (-1.0, -1.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (0.0, -1.0),
            (-1.0, 0.0),
            (3.0, 4.0),
            (-3.0, 4.0),
            (5.0, 12.0),
        ];
        for &(y, x) in cases {
            let got = Fixed::from_num(y).atan2(Fixed::from_num(x));
            assert_fixed_approx(got, y.atan2(x), tolerance);
        }
    }

    #[test]
    fn atan2_of_origin_is_zero_not_a_panic() {
        assert_eq!(
            Fixed::from_num(0.0).atan2(Fixed::from_num(0.0)),
            Fixed::ZERO
        );
    }

    #[test]
    fn clamp_min_max_behave_like_the_float_versions() {
        let lo = Fixed::from_num(-1.0);
        let hi = Fixed::from_num(1.0);
        assert_eq!(Fixed::from_num(5.0).clamp(lo, hi), hi);
        assert_eq!(Fixed::from_num(-5.0).clamp(lo, hi), lo);
        assert_eq!(Fixed::from_num(0.5).clamp(lo, hi), Fixed::from_num(0.5));
    }
}
