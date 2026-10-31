//! The float (`f32`) multivector: the general element of the algebra.

use core::ops::{Add, Mul, Neg, Sub};

use meridian_numeric::Scalar;

use super::basis_product::basis_product;
use crate::blade;

/// The general element of the algebra: closed under the geometric product.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Multivector(pub [Scalar; 16]);

impl Multivector {
    /// The zero multivector.
    pub const ZERO: Multivector = Multivector([0.0; 16]);

    /// Constructs a multivector containing only a scalar term.
    pub fn scalar(s: Scalar) -> Self {
        let mut m = [0.0; 16];
        m[blade::S] = s;
        Multivector(m)
    }

    /// Grade reversal: negates each blade by `(-1)^(k*(k-1)/2)` for its
    /// grade `k` (the number of basis vectors it's built from). Used to
    /// build the "conjugate" side of a sandwich product.
    pub fn reverse(self) -> Self {
        let mut out = self.0;
        for (i, v) in out.iter_mut().enumerate() {
            let k = (i as u32).count_ones();
            // grades 0,1 -> +1; grades 2,3 -> -1; grade 4 -> +1
            if matches!(k, 2 | 3) {
                *v = -*v;
            }
        }
        Multivector(out)
    }
}

impl Add for Multivector {
    type Output = Multivector;
    fn add(self, rhs: Multivector) -> Multivector {
        let mut out = self.0;
        for (o, r) in out.iter_mut().zip(rhs.0) {
            *o += r;
        }
        Multivector(out)
    }
}

impl Sub for Multivector {
    type Output = Multivector;
    fn sub(self, rhs: Multivector) -> Multivector {
        let mut out = self.0;
        for (o, r) in out.iter_mut().zip(rhs.0) {
            *o -= r;
        }
        Multivector(out)
    }
}

impl Neg for Multivector {
    type Output = Multivector;
    fn neg(self) -> Multivector {
        let mut out = self.0;
        for v in &mut out {
            *v = -*v;
        }
        Multivector(out)
    }
}

impl Mul<Scalar> for Multivector {
    type Output = Multivector;
    fn mul(self, rhs: Scalar) -> Multivector {
        let mut out = self.0;
        for v in &mut out {
            *v *= rhs;
        }
        Multivector(out)
    }
}

impl Mul for Multivector {
    type Output = Multivector;
    /// The geometric product — the one operation the whole algebra (and
    /// every type in this crate) is built from.
    fn mul(self, rhs: Multivector) -> Multivector {
        let mut out = [0.0 as Scalar; 16];
        for a in 0..16u8 {
            let lhs_val = self.0[a as usize];
            if lhs_val == 0.0 {
                continue;
            }
            for b in 0..16u8 {
                let rhs_val = rhs.0[b as usize];
                if rhs_val == 0.0 {
                    continue;
                }
                let (sign, mask) = basis_product(a, b);
                if sign == 0 {
                    continue;
                }
                out[mask as usize] += Scalar::from(sign) * lhs_val * rhs_val;
            }
        }
        Multivector(out)
    }
}
