//! The WGSL CORDIC sine/cosine library.

/// `sin_cos`/`atan2` via CORDIC — see the module doc for why this needs
/// no 64-bit emulation. Concatenated after [`FIXED_ARITHMETIC_LIB_WGSL`](super::FIXED_ARITHMETIC_LIB_WGSL)
/// in `dispatch_entry_points_wgsl` (the two constants are independent;
/// `pub` for the same cross-crate-reuse reason as
/// `FIXED_ARITHMETIC_LIB_WGSL`).
///
/// Constants below are the same values `meridian_numeric::fixed`
/// computes at runtime via `Fixed::from_num`/`cordic_atan_table`, taken
/// as literal Q16.16 bit patterns since they never change: `PI_BITS`/
/// `HALF_PI_BITS`/`TWO_PI_BITS`/`CORDIC_GAIN_BITS` match
/// `Fixed::pi()`/`half_pi()`/`two_pi()`/`CORDIC_GAIN` respectively, and
/// `CORDIC_ATAN_TABLE` matches `cordic_atan_table()` element-for-element
/// (`atan(2^-i)` for `i` in `0..24`, the same published table for `i <=
/// 15` and `2^-i` itself beyond that — see that function's doc comment).
/// [`FixedArithmeticKernels`](super::FixedArithmeticKernels)'s own tests below cross-check the whole
/// pipeline against the CPU `Fixed::sin_cos`/`atan2`, so any transcription
/// error here would fail bit-exactness, not silently drift.
pub const CORDIC_LIB_WGSL: &str = r#"
const CORDIC_ITERATIONS: u32 = 24u;

// Matches `CORDIC_GAIN` (0.607_252_935_008_881_2) rounded to Q16.16.
const CORDIC_GAIN_BITS: i32 = 39797;
// Matches `Fixed::pi()`/`half_pi()`/`two_pi()`.
const PI_BITS: i32 = 205887;
const HALF_PI_BITS: i32 = 102944;
const TWO_PI_BITS: i32 = 411775;

// Matches `cordic_atan_table()`: atan(2^-i) for i in 0..24, Q16.16.
const CORDIC_ATAN_TABLE = array<i32, 24>(
    51472, 30386, 16055, 8150, 4091, 2047, 1024, 512, 256, 128, 64, 32,
    16, 8, 4, 2, 1, 1, 0, 0, 0, 0, 0, 0
);

// Matches `reduce_to_pi_range`: `angle.rem_euclid(two_pi_bits)`, then
// fold anything past `pi_bits` back into `(-pi, pi]`. WGSL's `%` on i32
// takes the sign of the dividend (like Rust's `%`, not `rem_euclid`), so
// the euclidean fixup (`if r < 0 { r + TWO_PI_BITS }`) is done by hand.
fn reduce_to_pi_range(angle: i32) -> i32 {
    var r = angle % TWO_PI_BITS;
    if (r < 0) {
        r = r + TWO_PI_BITS;
    }
    if (r > PI_BITS) {
        return r - TWO_PI_BITS;
    }
    return r;
}

// Matches `cordic_rotate`: rotates (CORDIC_GAIN, 0) by `theta` (assumed
// in [-pi/2, pi/2]) via shift/add only. Returns (sin, cos) as
// vec2<i32>.x/.y, matching the Rust function's `(y, x)` return order.
// `x >> i`/`y >> i` is WGSL's arithmetic (sign-extending) right shift on
// i32 — identical to Rust's `i32 >> u32` — so this is a direct
// translation, not a reimplementation.
fn cordic_rotate(theta: i32) -> vec2<i32> {
    var x = CORDIC_GAIN_BITS;
    var y = 0;
    var z = theta;
    for (var i: u32 = 0u; i < CORDIC_ITERATIONS; i = i + 1u) {
        let angle = CORDIC_ATAN_TABLE[i];
        let x_shifted = x >> i;
        let y_shifted = y >> i;
        if (z >= 0) {
            let next_x = x - y_shifted;
            let next_y = y + x_shifted;
            x = next_x;
            y = next_y;
            z = z - angle;
        } else {
            let next_x = x + y_shifted;
            let next_y = y - x_shifted;
            x = next_x;
            y = next_y;
            z = z + angle;
        }
    }
    return vec2<i32>(y, x);
}

// Matches `Fixed::sin_cos`: quadrant-reduce into [-pi/2, pi/2] (folding
// the other half of the circle through `pi - reduced`/`-pi - reduced`
// and negating cos), then `cordic_rotate`. Returns (sin, cos).
fn fixed_sin_cos(theta: i32) -> vec2<i32> {
    let reduced = reduce_to_pi_range(theta);
    var t: i32;
    var negate_cos: bool;
    if (reduced > HALF_PI_BITS) {
        t = PI_BITS - reduced;
        negate_cos = true;
    } else if (reduced < -HALF_PI_BITS) {
        t = -PI_BITS - reduced;
        negate_cos = true;
    } else {
        t = reduced;
        negate_cos = false;
    }
    let sc = cordic_rotate(t);
    if (negate_cos) {
        return vec2<i32>(sc.x, -sc.y);
    }
    return sc;
}

// Matches `cordic_vector_angle`: rotates (x, y) toward the x-axis,
// accumulating the angle — converges to atan2(y, x) for x >= 0.
fn cordic_vector_angle(y_in: i32, x_in: i32) -> i32 {
    var cx = x_in;
    var cy = y_in;
    var z = 0;
    for (var i: u32 = 0u; i < CORDIC_ITERATIONS; i = i + 1u) {
        let angle = CORDIC_ATAN_TABLE[i];
        let x_shifted = cx >> i;
        let y_shifted = cy >> i;
        if (cy >= 0) {
            let next_x = cx + y_shifted;
            let next_y = cy - x_shifted;
            cx = next_x;
            cy = next_y;
            z = z + angle;
        } else {
            let next_x = cx - y_shifted;
            let next_y = cy + x_shifted;
            cx = next_x;
            cy = next_y;
            z = z - angle;
        }
    }
    return z;
}

// Matches `Fixed::atan2`: reflects x < 0 into the right half-plane (the
// only region `cordic_vector_angle` converges in) and corrects by +-pi.
fn fixed_atan2(y: i32, x: i32) -> i32 {
    if (x == 0 && y == 0) {
        return 0;
    }
    if (x < 0) {
        let base = cordic_vector_angle(-y, -x);
        if (y >= 0) {
            return base + PI_BITS;
        }
        return base - PI_BITS;
    }
    return cordic_vector_angle(y, x);
}
"#;
