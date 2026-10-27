//! The fixed-point (`Fixed`) multivector, mirroring [`super::float`].

use core::ops::{Add, Mul, Neg, Sub};

use meridian_numeric::Fixed;

use super::basis_product::basis_product;
use crate::blade;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedMultivector(pub [Fixed; 16]);

impl FixedMultivector {
    pub const ZERO: FixedMultivector = FixedMultivector([Fixed::ZERO; 16]);

    pub fn scalar(s: Fixed) -> Self {
        let mut m = [Fixed::ZERO; 16];
        m[blade::S] = s;
        FixedMultivector(m)
    }

    pub fn reverse(self) -> Self {
        let mut out = self.0;
        for (i, v) in out.iter_mut().enumerate() {
            let k = (i as u32).count_ones();
            if matches!(k, 2 | 3) {
                *v = -*v;
            }
        }
        FixedMultivector(out)
    }
}

impl Add for FixedMultivector {
    type Output = FixedMultivector;
    fn add(self, rhs: FixedMultivector) -> FixedMultivector {
        let mut out = self.0;
        for (o, r) in out.iter_mut().zip(rhs.0) {
            *o = *o + r;
        }
        FixedMultivector(out)
    }
}

impl Sub for FixedMultivector {
    type Output = FixedMultivector;
    fn sub(self, rhs: FixedMultivector) -> FixedMultivector {
        let mut out = self.0;
        for (o, r) in out.iter_mut().zip(rhs.0) {
            *o = *o - r;
        }
        FixedMultivector(out)
    }
}

impl Neg for FixedMultivector {
    type Output = FixedMultivector;
    fn neg(self) -> FixedMultivector {
        let mut out = self.0;
        for v in &mut out {
            *v = -*v;
        }
        FixedMultivector(out)
    }
}

impl Mul for FixedMultivector {
    type Output = FixedMultivector;
    fn mul(self, rhs: FixedMultivector) -> FixedMultivector {
        let mut out = [Fixed::ZERO; 16];
        for a in 0..16u8 {
            let lhs_val = self.0[a as usize];
            if lhs_val == Fixed::ZERO {
                continue;
            }
            for b in 0..16u8 {
                let rhs_val = rhs.0[b as usize];
                if rhs_val == Fixed::ZERO {
                    continue;
                }
                let (sign, mask) = basis_product(a, b);
                if sign == 0 {
                    continue;
                }
                let sign = if sign > 0 { Fixed::ONE } else { -Fixed::ONE };
                out[mask as usize] = out[mask as usize] + sign * lhs_val * rhs_val;
            }
        }
        FixedMultivector(out)
    }
}
