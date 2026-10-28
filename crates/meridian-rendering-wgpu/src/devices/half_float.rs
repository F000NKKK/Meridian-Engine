//! `f32` to IEEE half-precision bits, for `Rgba16Float` uploads.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "bit manipulation: each value is masked or range-checked to fit before the cast"
)]

/// The half-precision bits of `value`, rounded to nearest. Values beyond the
/// largest finite half (65504) clamp to it instead of becoming infinity, so one
/// overbright texel cannot poison a blur; NaN becomes a quiet NaN.
pub(crate) fn to_half(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let magnitude = bits & 0x7fff_ffff;
    if magnitude > 0x7f80_0000 {
        return sign | 0x7e00;
    }
    if magnitude >= 0x3880_0000 {
        let rounded = magnitude + 0x0fff + ((magnitude >> 13) & 1);
        let half = ((rounded >> 13).saturating_sub(112 << 10)).min(0x7bff) as u16;
        return sign | half;
    }
    let exponent = (magnitude >> 23) as i32 - 127;
    let shift = (-1 - exponent) as u32;
    if shift >= 25 {
        return sign;
    }
    let mantissa = (magnitude & 0x007f_ffff) | 0x0080_0000;
    sign | ((mantissa + (1 << (shift - 1))) >> shift) as u16
}

#[cfg(test)]
mod tests {
    use super::to_half;

    #[test]
    fn exact_values_encode_exactly() {
        assert_eq!(to_half(0.0), 0x0000);
        assert_eq!(to_half(1.0), 0x3c00);
        assert_eq!(to_half(0.5), 0x3800);
        assert_eq!(to_half(-2.0), 0xc000);
        assert_eq!(to_half(65504.0), 0x7bff);
    }

    #[test]
    fn overbright_values_clamp_and_tiny_values_flush() {
        assert_eq!(to_half(1.0e9), 0x7bff);
        assert_eq!(to_half(f32::INFINITY), 0x7bff);
        assert_eq!(to_half(1.0e-10), 0x0000);
        assert_eq!(to_half(2.0f32.powi(-24)), 0x0001);
        assert_eq!(to_half(f32::NAN) & 0x7c00, 0x7c00);
    }

    #[test]
    fn rounding_is_to_nearest() {
        // 1 + 2^-11 is halfway between two halves; 1 + 3 * 2^-12 rounds up.
        assert_eq!(to_half(1.0 + 3.0 * 2.0f32.powi(-12)), 0x3c01);
        assert_eq!(to_half(1.0 + 2.0f32.powi(-12)), 0x3c00);
    }
}
