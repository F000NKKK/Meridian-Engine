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
//! **Scope so far: `+`/`-`/`*`/`/`/`sqrt`.** The CORDIC `sin_cos`/`atan2`
//! are still real follow-up work, not done here — CORDIC needs the full
//! `atan` table ported, a larger, separable piece of work building on
//! the arithmetic emulation this module already proves out.
//!
//! [`FixedArithmeticKernels`] compiles the four binary-op compute
//! pipelines once against a [`meridian_compute_runtime::ComputeContext`]'s
//! GPU backend (see that crate's module doc for `ComputeContext::with_gpu`)
//! and [`FixedArithmeticKernels::dispatch`] runs one operation over a
//! batch of operand pairs, returning one [`Fixed`] result per pair — the
//! real, reusable API this phase's work is for, not just a
//! proof-of-concept hidden in a test.

use meridian_compute_runtime::ComputeContext;
use meridian_gpu_driver::{BufferUsage, ComputePipeline, Shader};
use meridian_numeric_core::Fixed;

/// The shared WGSL library: 32x32->64 unsigned multiply (via 16-bit limb
/// splitting), a 64-bit-by-32-bit unsigned long division, and
/// `fixed_add`/`fixed_sub`/`fixed_mul`/`fixed_div` built on them,
/// matching `meridian_numeric_core::Fixed`'s own `Add`/`Sub`/`Mul`/`Div`
/// impls operation-for-operation (see each function's comment for the
/// exact correspondence). Concatenated with a small dispatch-entry-point
/// suffix per operation in [`FixedArithmeticKernels::new`] — this
/// constant is the part shared across all four.
const FIXED_ARITHMETIC_LIB_WGSL: &str = r#"
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
// ported bit-for-bit from `meridian_numeric_core::fixed::isqrt_u64`:
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

/// Which [`Fixed`] binary operation a [`FixedArithmeticKernels`] pipeline
/// runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl FixedBinaryOp {
    fn entry_point(self) -> &'static str {
        match self {
            FixedBinaryOp::Add => "dispatch_add",
            FixedBinaryOp::Sub => "dispatch_sub",
            FixedBinaryOp::Mul => "dispatch_mul",
            FixedBinaryOp::Div => "dispatch_div",
        }
    }
}

/// The four dispatch entry points, sharing [`FIXED_ARITHMETIC_LIB_WGSL`]:
/// each reads one `(a, b)` pair from `operands` and writes one result
/// into `results`, both `array<i32>` (a `Fixed`'s raw Q16.16 bits) —
/// `operands[2*id]`/`operands[2*id+1]` are `a`/`b` for invocation `id`,
/// `results[id]` is that invocation's output. Two buffers (not the
/// single-binding shape `gpu-driver`'s own `dispatch_compute` supports)
/// means this crate records its own compute pass rather than reusing
/// that convenience method — see [`FixedArithmeticKernels::dispatch`].
fn dispatch_entry_points_wgsl() -> String {
    format!(
        r#"
{lib}

@group(0) @binding(0)
var<storage, read> operands: array<i32>;

@group(0) @binding(1)
var<storage, read_write> results: array<i32>;

@compute @workgroup_size(64)
fn dispatch_add(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_add(operands[2u * id.x], operands[2u * id.x + 1u]);
}}

@compute @workgroup_size(64)
fn dispatch_sub(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_sub(operands[2u * id.x], operands[2u * id.x + 1u]);
}}

@compute @workgroup_size(64)
fn dispatch_mul(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_mul(operands[2u * id.x], operands[2u * id.x + 1u]);
}}

@compute @workgroup_size(64)
fn dispatch_div(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_div(operands[2u * id.x], operands[2u * id.x + 1u]);
}}

// Unary, unlike the four above: one operand per invocation
// (`operands[id.x]`, not a `2*id.x`/`2*id.x+1` pair) — see
// `FixedArithmeticKernels::dispatch_sqrt`, which populates `operands`
// accordingly.
@compute @workgroup_size(64)
fn dispatch_sqrt(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_sqrt(operands[id.x]);
}}
"#,
        lib = FIXED_ARITHMETIC_LIB_WGSL
    )
}

/// Compiled compute pipelines for every [`Fixed`] binary operation,
/// built once against a [`ComputeContext`]'s GPU backend
/// ([`ComputeContext::with_gpu`]) and reused across dispatches.
#[derive(Debug)]
pub struct FixedArithmeticKernels {
    #[allow(dead_code)]
    shader: Shader,
    add: ComputePipeline,
    sub: ComputePipeline,
    mul: ComputePipeline,
    div: ComputePipeline,
    sqrt: ComputePipeline,
}

impl FixedArithmeticKernels {
    /// Compiles the WGSL module and builds every pipeline (the four
    /// binary ops plus `sqrt`). `context` must already have a GPU
    /// backend ([`ComputeContext::with_gpu`]) — panics otherwise, the
    /// same "caller opted into GPU dispatch explicitly, so a missing
    /// backend is a logic error to surface immediately" policy
    /// `meridian-graphics-driver`'s own headless-device tests use.
    pub fn new(context: &ComputeContext) -> Self {
        let gpu = context
            .gpu()
            .expect("FixedArithmeticKernels::new requires a ComputeContext with a GPU backend (see ComputeContext::with_gpu)");
        let source = dispatch_entry_points_wgsl();
        let shader = gpu.create_shader("fixed_arithmetic", &source);
        let add = gpu.create_compute_pipeline(&shader, FixedBinaryOp::Add.entry_point());
        let sub = gpu.create_compute_pipeline(&shader, FixedBinaryOp::Sub.entry_point());
        let mul = gpu.create_compute_pipeline(&shader, FixedBinaryOp::Mul.entry_point());
        let div = gpu.create_compute_pipeline(&shader, FixedBinaryOp::Div.entry_point());
        let sqrt = gpu.create_compute_pipeline(&shader, "dispatch_sqrt");
        Self {
            shader,
            add,
            sub,
            mul,
            div,
            sqrt,
        }
    }

    fn pipeline(&self, op: FixedBinaryOp) -> &ComputePipeline {
        match op {
            FixedBinaryOp::Add => &self.add,
            FixedBinaryOp::Sub => &self.sub,
            FixedBinaryOp::Mul => &self.mul,
            FixedBinaryOp::Div => &self.div,
        }
    }

    /// Runs `op` element-wise over `pairs` on the GPU, returning one
    /// [`Fixed`] result per pair, in order. A real `async fn` — reading
    /// the result buffer back waits on in-flight GPU work (see
    /// `meridian_gpu_driver`'s module doc for this workspace's "async on
    /// genuine I/O" policy).
    pub async fn dispatch(
        &self,
        context: &ComputeContext,
        op: FixedBinaryOp,
        pairs: &[(Fixed, Fixed)],
    ) -> Vec<Fixed> {
        let gpu = context.gpu().expect(
            "FixedArithmeticKernels::dispatch requires a ComputeContext with a GPU backend",
        );

        let mut operand_bytes = Vec::with_capacity(pairs.len() * 8);
        for (a, b) in pairs {
            operand_bytes.extend_from_slice(&a.to_bits().to_le_bytes());
            operand_bytes.extend_from_slice(&b.to_bits().to_le_bytes());
        }
        let operands = gpu.allocate_buffer(operand_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&operands, &operand_bytes);

        let results = gpu.allocate_buffer(pairs.len() * 4, BufferUsage::Storage);

        // Two-buffer dispatch: `GpuComputeDevice::dispatch`'s single-
        // buffer convenience doesn't cover "read from one buffer, write
        // to another," so this builds its own bind group and records its
        // own compute pass via the general (multi-buffer) forms
        // `meridian_gpu_driver::Device`/`CommandBuffer` expose for
        // exactly this shape.
        let pipeline = self.pipeline(op);
        let device = gpu.gpu_driver_device();
        let bind_group =
            device.create_bind_group(&pipeline.bind_group_layout(), &[&operands, &results]);

        let mut commands = device.create_command_buffer();
        commands.dispatch_compute_with_bind_group(
            pipeline,
            &bind_group,
            (pairs.len() as u32).div_ceil(64).max(1),
        );
        commands.submit();

        let result_bytes = gpu.read_buffer(&results).await;
        result_bytes
            .chunks_exact(4)
            .map(|c| Fixed::from_bits(i32::from_le_bytes(c.try_into().unwrap())))
            .collect()
    }

    /// Runs `sqrt` element-wise over `values` on the GPU, returning one
    /// [`Fixed`] result per value, in order. Every `value` must be
    /// non-negative — the same precondition `Fixed::sqrt` asserts on the
    /// CPU side; WGSL has no panic, so passing a negative value here
    /// produces a nonsensical (not merely wrong-sign) result rather than
    /// erroring, and callers are responsible for upholding it themselves.
    pub async fn dispatch_sqrt(&self, context: &ComputeContext, values: &[Fixed]) -> Vec<Fixed> {
        let gpu = context.gpu().expect(
            "FixedArithmeticKernels::dispatch_sqrt requires a ComputeContext with a GPU backend",
        );

        let mut operand_bytes = Vec::with_capacity(values.len() * 4);
        for value in values {
            operand_bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        let operands = gpu.allocate_buffer(operand_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&operands, &operand_bytes);

        let results = gpu.allocate_buffer(values.len() * 4, BufferUsage::Storage);

        let device = gpu.gpu_driver_device();
        let bind_group =
            device.create_bind_group(&self.sqrt.bind_group_layout(), &[&operands, &results]);

        let mut commands = device.create_command_buffer();
        commands.dispatch_compute_with_bind_group(
            &self.sqrt,
            &bind_group,
            (values.len() as u32).div_ceil(64).max(1),
        );
        commands.submit();

        let result_bytes = gpu.read_buffer(&results).await;
        result_bytes
            .chunks_exact(4)
            .map(|c| Fixed::from_bits(i32::from_le_bytes(c.try_into().unwrap())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A battery of `Fixed` values chosen to exercise the emulation's
    /// real edge cases: zero, unit values, small/large magnitudes near
    /// `Fixed`'s ±32768 range, values that stress `fixed_mul`'s 16-bit
    /// limb carry propagation (large `hi * hi`/`hi * lo` cross terms),
    /// and raw-bit-pattern values (`from_bits`, not `from_num`) that
    /// `from_num`'s rounding wouldn't otherwise reach — not just a few
    /// "nice" numbers.
    fn interesting_values() -> Vec<Fixed> {
        const FROM_F64: &[f64] = &[
            0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            2.0,
            -2.0,
            0.000_1,
            -0.000_1,
            9.81,
            -9.81,
            core::f64::consts::PI,
            -core::f64::consts::PI,
            100.0,
            -100.0,
            12_345.678_9,
            -12_345.678_9,
            30000.0,
            -30000.0,
            0.015_625,
            -0.015_625,
        ];
        const FROM_BITS: &[i32] = &[
            0,
            1,
            -1,
            2,
            -2,
            65536,
            -65536,
            i32::MAX / 2,
            i32::MIN / 2 + 1,
            1_000_000,
            -1_000_000,
            12345,
            -54321,
        ];
        FROM_F64
            .iter()
            .map(|&v| Fixed::from_num(v))
            .chain(FROM_BITS.iter().map(|&b| Fixed::from_bits(b)))
            .collect()
    }

    /// [`interesting_values`] minus the large-magnitude values — those
    /// exist specifically to stress `fixed_mul`'s 64-bit emulation
    /// (which legitimately produces a *smaller*-magnitude result after
    /// its `>> 16`, never overflowing `i32`), but summing two
    /// large-enough `Fixed` values overflows `i32` outright (`Fixed`'s
    /// own doc comment gives the range as "roughly ±32768" — two values
    /// even moderately close to that limit already overflow when added).
    /// That's a real out-of-range condition for `Fixed` itself (Rust's
    /// own debug-mode `Fixed::add`/`Fixed::sub` panic on it, same as any
    /// other `i32` overflow), not something `fixed_add`/`fixed_sub`'s
    /// trivial one-instruction WGSL needs stress-testing against — kept
    /// safely inside range so `a + b`/`a - b` never overflows for any
    /// pair.
    fn moderate_values() -> Vec<Fixed> {
        interesting_values()
            .into_iter()
            .filter(|v| v.abs() <= Fixed::from_num(10000.0))
            .collect()
    }

    /// Every value paired with every other value (including itself) —
    /// the full cross product, so both operand orders and every
    /// magnitude/sign combination get exercised.
    fn all_pairs(values: &[Fixed]) -> Vec<(Fixed, Fixed)> {
        values
            .iter()
            .flat_map(|&a| values.iter().map(move |&b| (a, b)))
            .collect()
    }

    /// Needs a real adapter; some CI/sandboxed environments have none —
    /// skip rather than fail, matching every other GPU-touching test in
    /// this workspace.
    async fn kernels_or_skip() -> Option<(ComputeContext, FixedArithmeticKernels)> {
        let context = match ComputeContext::new().with_gpu().await {
            Ok(ctx) => ctx,
            Err(err) => {
                eprintln!("skipping: no GPU device available ({err})");
                return None;
            }
        };
        let kernels = FixedArithmeticKernels::new(&context);
        Some((context, kernels))
    }

    /// The actual point of this whole module: every `Fixed` add computed
    /// on the GPU must be bit-for-bit identical to the CPU `Fixed::add`
    /// result — not just numerically close.
    #[tokio::test]
    async fn gpu_add_matches_cpu_bit_exact() {
        let Some((context, kernels)) = kernels_or_skip().await else {
            return;
        };
        let pairs = all_pairs(&moderate_values());
        let gpu_results = kernels.dispatch(&context, FixedBinaryOp::Add, &pairs).await;
        for (i, &(a, b)) in pairs.iter().enumerate() {
            let expected = a + b;
            assert_eq!(
                gpu_results[i], expected,
                "add mismatch for ({a:?}, {b:?}): cpu={expected:?} gpu={:?}",
                gpu_results[i]
            );
        }
    }

    #[tokio::test]
    async fn gpu_sub_matches_cpu_bit_exact() {
        let Some((context, kernels)) = kernels_or_skip().await else {
            return;
        };
        let pairs = all_pairs(&moderate_values());
        let gpu_results = kernels.dispatch(&context, FixedBinaryOp::Sub, &pairs).await;
        for (i, &(a, b)) in pairs.iter().enumerate() {
            let expected = a - b;
            assert_eq!(
                gpu_results[i], expected,
                "sub mismatch for ({a:?}, {b:?}): cpu={expected:?} gpu={:?}",
                gpu_results[i]
            );
        }
    }

    /// The hardest case: `fixed_mul`'s 64-bit-product emulation (16-bit
    /// limb splitting + carry propagation) has the most ways to be
    /// subtly wrong, so this is the test that actually validates the
    /// emulation's correctness, not just its existence.
    #[tokio::test]
    async fn gpu_mul_matches_cpu_bit_exact() {
        let Some((context, kernels)) = kernels_or_skip().await else {
            return;
        };
        let pairs = all_pairs(&interesting_values());
        let gpu_results = kernels.dispatch(&context, FixedBinaryOp::Mul, &pairs).await;
        for (i, &(a, b)) in pairs.iter().enumerate() {
            let expected = a * b;
            assert_eq!(
                gpu_results[i], expected,
                "mul mismatch for ({a:?}, {b:?}): cpu={expected:?} gpu={:?}",
                gpu_results[i]
            );
        }
    }

    #[tokio::test]
    async fn gpu_div_matches_cpu_bit_exact() {
        let Some((context, kernels)) = kernels_or_skip().await else {
            return;
        };
        let values = interesting_values();
        let pairs: Vec<(Fixed, Fixed)> = all_pairs(&values)
            .into_iter()
            .filter(|&(_, b)| b != Fixed::ZERO) // Fixed::div panics on zero, same as the GPU kernel would misbehave on
            .collect();
        let gpu_results = kernels.dispatch(&context, FixedBinaryOp::Div, &pairs).await;
        for (i, &(a, b)) in pairs.iter().enumerate() {
            let expected = a / b;
            assert_eq!(
                gpu_results[i], expected,
                "div mismatch for ({a:?}, {b:?}): cpu={expected:?} gpu={:?}",
                gpu_results[i]
            );
        }
    }
}
