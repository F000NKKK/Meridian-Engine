//! Tests of this module.
#![cfg(feature = "hw-tests")]

use meridian_compute_app::ComputeContext;
use meridian_compute_wgpu::gpu_context;
use meridian_numeric::Fixed;

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
#[cfg(feature = "hw-tests")]
async fn kernels_or_skip() -> Option<(ComputeContext, FixedArithmeticKernels)> {
    let context = match gpu_context().await {
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
#[cfg(feature = "hw-tests")]
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

#[cfg(feature = "hw-tests")]
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
#[cfg(feature = "hw-tests")]
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

#[cfg(feature = "hw-tests")]
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

/// [`isqrt_u64`]'s Newton loop iterates a variable number of times
/// depending on `n`, so this is the test that actually validates the
/// WGSL port converges to the same fixed point as the CPU version,
/// not just that it terminates for a couple of easy inputs.
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn gpu_sqrt_matches_cpu_bit_exact() {
    let Some((context, kernels)) = kernels_or_skip().await else {
        return;
    };
    let values: Vec<Fixed> = interesting_values()
        .into_iter()
        .filter(|v| v.to_bits() >= 0) // Fixed::sqrt panics on negative
        .collect();
    let gpu_results = kernels.dispatch_sqrt(&context, &values).await;
    for (i, &v) in values.iter().enumerate() {
        let expected = v.sqrt();
        assert_eq!(
            gpu_results[i], expected,
            "sqrt mismatch for {v:?}: cpu={expected:?} gpu={:?}",
            gpu_results[i]
        );
    }
}

/// A spread of angles covering multiple full turns in both
/// directions and every quadrant-reduction branch in
/// `fixed_sin_cos`/`Fixed::sin_cos` (inside `[-pi/2, pi/2]`, and past
/// it on both sides, forcing the `pi - reduced`/`-pi - reduced`
/// negate-cos branches).
fn interesting_angles() -> Vec<Fixed> {
    let mut degrees = -720i32;
    let mut angles = Vec::new();
    while degrees <= 720 {
        let radians = f64::from(degrees) * core::f64::consts::PI / 180.0;
        angles.push(Fixed::from_num(radians));
        degrees += 15;
    }
    angles
}

/// The whole point of the CORDIC port: every `sin_cos` computed on
/// the GPU must be bit-for-bit identical to the CPU
/// `Fixed::sin_cos` result, across the full quadrant-reduction
/// state machine, not just the `[-pi/2, pi/2]` core CORDIC loop.
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn gpu_sin_cos_matches_cpu_bit_exact() {
    let Some((context, kernels)) = kernels_or_skip().await else {
        return;
    };
    let angles = interesting_angles();
    let gpu_results = kernels.dispatch_sin_cos(&context, &angles).await;
    for (i, &theta) in angles.iter().enumerate() {
        let expected = theta.sin_cos();
        assert_eq!(
            gpu_results[i], expected,
            "sin_cos mismatch for {theta:?}: cpu={expected:?} gpu={:?}",
            gpu_results[i]
        );
    }
}

/// Same bit-exactness bar for `atan2`, across every quadrant
/// (including the `x < 0` reflection branch) and the origin
/// special case. Uses `moderate_values`, not `interesting_values`:
/// `Fixed::atan2`'s `x < 0` branch negates and re-adds `pi`, which
/// overflows `i32` for the large-magnitude values `interesting_values`
/// includes to stress `fixed_mul` — a real limitation of the existing
/// CPU `Fixed::atan2`, not something this GPU port needs to
/// reproduce a bug-for-bug match for.
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn gpu_atan2_matches_cpu_bit_exact() {
    let Some((context, kernels)) = kernels_or_skip().await else {
        return;
    };
    let values = moderate_values();
    let pairs = all_pairs(&values);
    let gpu_results = kernels
        .dispatch(&context, FixedBinaryOp::Atan2, &pairs)
        .await;
    for (i, &(y, x)) in pairs.iter().enumerate() {
        let expected = y.atan2(x);
        assert_eq!(
            gpu_results[i], expected,
            "atan2 mismatch for (y={y:?}, x={x:?}): cpu={expected:?} gpu={:?}",
            gpu_results[i]
        );
    }
}
