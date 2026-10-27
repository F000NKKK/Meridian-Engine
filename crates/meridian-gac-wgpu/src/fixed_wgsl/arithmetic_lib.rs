//! The WGSL Q16.16 arithmetic library.

/// The shared WGSL library: 32x32->64 unsigned multiply (via 16-bit limb
/// splitting), a 64-bit-by-64-bit unsigned long division, and
/// `fixed_add`/`fixed_sub`/`fixed_mul`/`fixed_div`/`fixed_sqrt` built on
/// them, matching `meridian_numeric::Fixed`'s own `Add`/`Sub`/`Mul`/
/// `Div`/`sqrt` impls operation-for-operation (see each function's
/// comment for the exact correspondence). Concatenated with a small
/// dispatch-entry-point suffix per operation in
/// [`FixedArithmeticKernels::new`](super::FixedArithmeticKernels::new) — this constant is the part shared
/// across all five, and `pub` so other GPU-dispatching adapter crates
/// (e.g. `meridian-physics-wgpu`'s deterministic soft-body kernel,
/// which needs `fixed_add`/`fixed_mul`/`fixed_sqrt` for spring-length
/// math) can concatenate it into their own WGSL modules instead of
/// re-deriving the same emulation.
pub const FIXED_ARITHMETIC_LIB_WGSL: &str = r#"
// A 64-bit unsigned integer as two 32-bit words (WGSL has no native
// 64-bit integer type).
struct U64 {
    hi: u32,
    lo: u32,
}

// Unsigned 32x32->64 multiply via 16-bit limb splitting: split each
// operand into high/low 16-bit halves (each fits exactly in a u32 with
// no overflow when multiplied), then combine the four partial products
// with explicit carry propagation. Standard technique for emulating a
// wide multiply with only native-width arithmetic.
fn u32_mul_wide(a: u32, b: u32) -> U64 {
    let a_lo = a & 0xFFFFu;
    let a_hi = a >> 16u;
    let b_lo = b & 0xFFFFu;
    let b_hi = b >> 16u;

    let lo_lo = a_lo * b_lo;
    let lo_hi = a_lo * b_hi;
    let hi_lo = a_hi * b_lo;
    let hi_hi = a_hi * b_hi;

    // result = hi_hi * 2^32 + (lo_hi + hi_lo) * 2^16 + lo_lo, with carries.
    let mid = lo_hi + hi_lo;
    let mid_carry = select(0u, 1u, mid < lo_hi); // mid overflowed u32

    let lo = lo_lo + (mid << 16u);
    let lo_carry = select(0u, 1u, lo < lo_lo);

    let hi = hi_hi + (mid >> 16u) + (mid_carry << 16u) + lo_carry;

    return U64(hi, lo);
}

// Two's complement negation of a 64-bit value held as two 32-bit words.
fn u64_negate(v: U64) -> U64 {
    let inv_lo = ~v.lo;
    let inv_hi = ~v.hi;
    let new_lo = inv_lo + 1u;
    let carry = select(0u, 1u, new_lo < inv_lo);
    let new_hi = inv_hi + carry;
    return U64(new_hi, new_lo);
}

// Unsigned 64-bit (as two u32 words) divided by an unsigned 32-bit
// divisor, truncating toward zero, keeping only the low 32 bits of the
// quotient (matching `as i32` truncation on the Rust side — every real
// `Fixed` division result fits in i32 range). Restoring binary long
// division, one bit at a time, 64 iterations (32 for the high word, 32
// for the low word) — correct for any numerator/divisor magnitude that
// keeps the running remainder under 2^32, which holds for every `Fixed`
// value this workspace produces (divisor magnitude well under 2^31 in
// practice for physics-scale quantities).
fn u64_div_u32(numerator: U64, divisor: u32) -> u32 {
    var remainder: u32 = 0u;
    var quotient: u32 = 0u;
    for (var i: i32 = 31; i >= 0; i = i - 1) {
        let bit = (numerator.hi >> u32(i)) & 1u;
        remainder = (remainder << 1u) | bit;
        if (remainder >= divisor) {
            remainder = remainder - divisor;
            quotient = (quotient << 1u) | 1u;
        } else {
            quotient = quotient << 1u;
        }
    }
    for (var i: i32 = 31; i >= 0; i = i - 1) {
        let bit = (numerator.lo >> u32(i)) & 1u;
        remainder = (remainder << 1u) | bit;
        if (remainder >= divisor) {
            remainder = remainder - divisor;
            quotient = (quotient << 1u) | 1u;
        } else {
            quotient = quotient << 1u;
        }
    }
    return quotient;
}

// Matches `impl Add for Fixed`: `Fixed(self.0 + rhs.0)`, plain i32
// addition (two's complement wraparound on overflow, same as Rust in a
// release build — real `Fixed` values stay well inside i32 range so this
// never matters in practice).
fn fixed_add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Matches `impl Sub for Fixed`: `Fixed(self.0 - rhs.0)`.
fn fixed_sub(a: i32, b: i32) -> i32 {
    return a - b;
}

// Matches `impl Mul for Fixed`: `(self.0 as i64 * rhs.0 as i64) >> 16`,
// truncated to i32. Computed via sign/magnitude: multiply the absolute
// values as unsigned 64-bit (exact, no truncation — 32x32 always fits in
// 64 bits), negate if the signs differed, then extract bits [16:48) of
// the 64-bit product. That bit range is exactly what ">> 16, keep low
// 32 bits" selects, and — because it's pure bit extraction from an
// already-correctly-signed two's complement value — this is valid
// whether the product is positive or negative, no separate arithmetic-
// shift-vs-logical-shift case needed.
fn fixed_mul(a: i32, b: i32) -> i32 {
    let negative = (a < 0) != (b < 0);
    let ua = u32(abs(a));
    let ub = u32(abs(b));
    var product = u32_mul_wide(ua, ub);
    if (negative) {
        product = u64_negate(product);
    }
    let result_bits = (product.hi << 16u) | (product.lo >> 16u);
    return bitcast<i32>(result_bits);
}

// Matches `impl Div for Fixed`: `((self.0 as i64) << 16) / rhs.0 as i64`,
// truncated toward zero, cast to i32. `(self.0 as i64) << 16` is built
// directly as a 64-bit value (`hi = |self.0| >> 16, lo = |self.0| << 16`
// — exactly what left-shifting a zero-extended value by 16 produces,
// split into words), then divided by `|rhs.0|` via `u64_div_u32`, with
// the sign reapplied at the end (integer division truncates toward
// zero, which sign/magnitude division does by construction).
fn fixed_div(a: i32, b: i32) -> i32 {
    let negative = (a < 0) != (b < 0);
    let ua = u32(abs(a));
    let ub = u32(abs(b));
    let numerator = U64(ua >> 16u, ua << 16u);
    let quotient = u64_div_u32(numerator, ub);
    let result = i32(quotient);
    return select(result, -result, negative);
}

// 64-bit (as two u32 words) addition, with carry out of the low word
// propagated into the high word.
fn u64_add(a: U64, b: U64) -> U64 {
    let lo = a.lo + b.lo;
    let carry = select(0u, 1u, lo < a.lo);
    let hi = a.hi + b.hi + carry;
    return U64(hi, lo);
}

// 64-bit subtraction, with borrow out of the low word propagated into
// the high word. Only ever called here with `a >= b` (restoring
// division's own invariant), so underflow of the high word never
// happens in practice.
fn u64_sub(a: U64, b: U64) -> U64 {
    let borrow = select(0u, 1u, a.lo < b.lo);
    let lo = a.lo - b.lo;
    let hi = a.hi - b.hi - borrow;
    return U64(hi, lo);
}

// Logical right shift by one bit, carrying the low bit of the high word
// into the top bit of the low word.
fn u64_shr1(v: U64) -> U64 {
    let lo = (v.lo >> 1u) | (v.hi << 31u);
    let hi = v.hi >> 1u;
    return U64(hi, lo);
}

// Logical left shift by one bit.
fn u64_shl1(v: U64) -> U64 {
    let hi = (v.hi << 1u) | (v.lo >> 31u);
    let lo = v.lo << 1u;
    return U64(hi, lo);
}

// Sets bit 0 of `v` to `bit` (0 or 1) — used by `u64_div_u64` to build up
// a shifted-in quotient/remainder bit at a time.
fn u64_or_lsb(v: U64, bit: u32) -> U64 {
    return U64(v.hi, v.lo | bit);
}

// Bit `i` (0 = least significant, up to 63) of a 64-bit value.
fn u64_get_bit(v: U64, i: u32) -> u32 {
    if (i >= 32u) {
        return (v.hi >> (i - 32u)) & 1u;
    }
    return (v.lo >> i) & 1u;
}

fn u64_ge(a: U64, b: U64) -> bool {
    if (a.hi != b.hi) {
        return a.hi > b.hi;
    }
    return a.lo >= b.lo;
}

fn u64_lt(a: U64, b: U64) -> bool {
    return !u64_ge(a, b);
}

fn u64_is_zero(v: U64) -> bool {
    return v.hi == 0u && v.lo == 0u;
}

// Unsigned 64-bit divided by unsigned 64-bit, truncating toward zero,
// keeping only the quotient (the remainder is discarded — nothing here
// needs it). Restoring binary long division, 64 iterations, generalizing
// `u64_div_u32` to a 64-bit divisor: `isqrt_u64`'s Newton iteration
// divides by its own running estimate `x`, which starts at `n` itself
// and can exceed 32 bits, so the narrower `u64_div_u32` doesn't cover it.
fn u64_div_u64(numerator: U64, divisor: U64) -> U64 {
    var remainder = U64(0u, 0u);
    var quotient = U64(0u, 0u);
    for (var i: i32 = 63; i >= 0; i = i - 1) {
        let bit = u64_get_bit(numerator, u32(i));
        remainder = u64_or_lsb(u64_shl1(remainder), bit);
        if (u64_ge(remainder, divisor)) {
            remainder = u64_sub(remainder, divisor);
            quotient = u64_or_lsb(u64_shl1(quotient), 1u);
        } else {
            quotient = u64_shl1(quotient);
        }
    }
    return quotient;
}

// Floor integer square root via Newton's method (Heron's method),
// ported bit-for-bit from `meridian_numeric::fixed::isqrt_u64`:
// starts from `n` itself and iterates `x -> (x + n/x) / 2` until it
// stops decreasing. `y = x.div_ceil(2)` on the Rust side is
// `(x + 1) >> 1` here (equivalent for all `x >= 0`, and `U64(0u, 1u)`
// never overflows `u64_add` since `n >= 1` in this branch).
fn isqrt_u64(n: U64) -> U64 {
    if (u64_is_zero(n)) {
        return U64(0u, 0u);
    }
    var x = n;
    var y = u64_shr1(u64_add(x, U64(0u, 1u)));
    loop {
        if (!u64_lt(y, x)) {
            break;
        }
        x = y;
        let q = u64_div_u64(n, x);
        y = u64_shr1(u64_add(x, q));
    }
    return x;
}

// Matches `Fixed::sqrt`: `Fixed(isqrt_u64((self.0 as u64) << 16) as i32)`.
// Callers must only pass non-negative `a` (the CPU-side `Fixed::sqrt`
// asserts this and panics otherwise; WGSL has no panic, so this is the
// caller's responsibility — see `FixedArithmeticKernels::dispatch_sqrt`).
// `(self.0 as u64) << 16` is built directly as a 64-bit value the same
// way `fixed_div`'s numerator is: `hi = a >> 16, lo = a << 16`.
fn fixed_sqrt(a: i32) -> i32 {
    let magnitude = u32(a);
    let scaled = U64(magnitude >> 16u, magnitude << 16u);
    let result = isqrt_u64(scaled);
    return bitcast<i32>(result.lo);
}
"#;
