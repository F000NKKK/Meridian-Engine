//! A single graded term of a multivector.

use meridian_numeric_core::Scalar;

/// A single k-vector term (scalar, vector, bivector, ...).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Blade {
    pub grade: u8,
    pub value: Scalar,
}
