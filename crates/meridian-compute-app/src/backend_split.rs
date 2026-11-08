//! How a hybrid dispatch divides items between the CPU and the GPU.

/// How [`ComputeContext::run_hybrid`](crate::ComputeContext::run_hybrid) splits `count` work items between
/// CPU and GPU.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackendSplit {
    /// Every item on the CPU.
    CpuOnly,
    /// Every item on the GPU.
    GpuOnly,
    /// `0.0..=1.0`: that fraction of items on the GPU, the rest on the
    /// CPU. `Ratio(0.5)` is an even 50/50 split. Out-of-range values are
    /// clamped, not a logic error — a caller computing a ratio
    /// dynamically (e.g. from a benchmark) shouldn't have to re-clamp it
    /// themselves.
    Ratio(f32),
}

impl BackendSplit {
    pub(crate) fn gpu_item_count(self, count: usize, gpu_available: bool) -> usize {
        if !gpu_available {
            return 0;
        }
        match self {
            BackendSplit::CpuOnly => 0,
            BackendSplit::GpuOnly => count,
            BackendSplit::Ratio(fraction) => {
                let fraction = fraction.clamp(0.0, 1.0);
                rounded_ratio_count(count, fraction)
            }
        }
    }
}

/// Rounds count * ratio using the exact binary representation of an f32.
/// This avoids lossy casts through f32/usize for large workloads.
fn rounded_ratio_count(count: usize, fraction: f32) -> usize {
    if !fraction.is_finite() || fraction <= 0.0 {
        return 0;
    }
    if fraction >= 1.0 {
        return count;
    }

    let bits = fraction.to_bits();
    let raw_exponent = (bits >> 23) & 0xff;
    let fractional_bits = bits & 0x7f_ffff;
    let significand = if raw_exponent == 0 {
        fractional_bits
    } else {
        fractional_bits | (1 << 23)
    };
    let exponent = if raw_exponent == 0 { 1 } else { raw_exponent };
    // ratio = significand / 2^(150 - exponent) for 0 < ratio < 1.
    let shift = 150 - exponent;
    if shift >= 128 {
        return 0;
    }
    let product = u128::try_from(count).expect("usize fits into u128")
        * u128::from(significand);
    let rounded = (product + (1_u128 << (shift - 1))) >> shift;
    usize::try_from(rounded).expect("ratio does not exceed original count")
}

#[cfg(test)]
mod ratio_tests {
    use super::{BackendSplit, rounded_ratio_count};

    #[test]
    fn boundary_ratios_and_missing_gpu() {
        assert_eq!(rounded_ratio_count(101, 0.0), 0);
        assert_eq!(rounded_ratio_count(101, 1.0), 101);
        assert_eq!(rounded_ratio_count(101, 0.5), 51);
        assert_eq!(rounded_ratio_count(101, f32::NAN), 0);
        assert_eq!(BackendSplit::Ratio(0.75).gpu_item_count(100, false), 0);
    }

    #[test]
    fn preserves_large_work_counts_without_f32_precision_loss() {
        let large = usize::MAX - 1024;
        assert_eq!(rounded_ratio_count(large, 0.5), large / 2 + 1);
    }
}
