//! [`StoredScalar`]: a component scalar of a stored GA value.

use substrate_data::Schema;

use crate::flavor::ScalarLike;

/// A component scalar of a stored GA value: `f32` or `Fixed`.
pub(crate) trait StoredScalar: ScalarLike + Schema {
    /// Whether `self` and `other` are the same bits (`-0.0` is not `0.0`).
    fn same_bits(self, other: Self) -> bool;

    /// Whether every component of `a` has the same bits as `b`'s.
    fn all_same_bits<const N: usize>(a: [Self; N], b: [Self; N]) -> bool {
        a.into_iter().zip(b).all(|(a, b)| a.same_bits(b))
    }
}
