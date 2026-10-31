//! A single graded term of a multivector.

use meridian_numeric::Scalar;

/// A single k-vector term (scalar, vector, bivector, ...).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Blade {
    /// Grade of the term: scalar is zero, vector is one, and so on.
    pub grade: u8,
    /// Scalar coefficient of the term.
    pub value: Scalar,
}
