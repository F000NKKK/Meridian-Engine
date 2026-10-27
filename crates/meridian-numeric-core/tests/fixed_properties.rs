//! Property tests for the deterministic `Fixed` type: algebraic laws that
//! must hold for every in-range operand, not just hand-picked ones.

use meridian_numeric_core::Fixed;
use proptest::prelude::*;

const SAFE: i32 = 1 << 28;

fn fixed() -> impl Strategy<Value = Fixed> {
    (-SAFE..SAFE).prop_map(Fixed::from_bits)
}

proptest! {
    #[test]
    fn addition_is_commutative(a in fixed(), b in fixed()) {
        prop_assert_eq!(a + b, b + a);
    }

    #[test]
    fn subtraction_inverts_addition(a in fixed(), b in fixed()) {
        prop_assert_eq!((a + b) - b, a);
    }

    #[test]
    fn double_negation_is_identity(a in fixed()) {
        prop_assert_eq!(-(-a), a);
    }

    #[test]
    fn abs_is_never_negative(a in fixed()) {
        prop_assert!(a.abs() >= Fixed::ZERO);
    }

    #[test]
    fn multiplying_by_one_is_identity(a in fixed()) {
        prop_assert_eq!(a * Fixed::ONE, a);
    }

    #[test]
    fn clamp_stays_within_bounds(a in fixed(), b in fixed(), c in fixed()) {
        let (lo, hi) = if b <= c { (b, c) } else { (c, b) };
        let clamped = a.clamp(lo, hi);
        prop_assert!(clamped >= lo && clamped <= hi);
    }

    #[test]
    fn float_round_trip_is_within_one_step(value in -4096.0f64..4096.0) {
        let back = Fixed::from_num(value).to_num();
        prop_assert!((back - value).abs() <= 1.0 / 65536.0);
    }

    #[test]
    fn sin_cos_lie_on_the_unit_circle(bits in -(1i32 << 20)..(1i32 << 20)) {
        let (sin, cos) = Fixed::from_bits(bits).sin_cos();
        let radius = sin.to_num().powi(2) + cos.to_num().powi(2);
        prop_assert!((radius - 1.0).abs() < 1e-2, "radius {radius}");
    }
}
