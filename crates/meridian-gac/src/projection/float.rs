//! The float-flavor projection alias and column-major `mat4` multiply.

use meridian_numeric::Scalar;

use crate::flavor::float::FloatFlavor;

pub type Projection = super::Projection<FloatFlavor>;

/// Column-major 4x4 matrix multiply (`lhs * rhs`, column-vector
/// convention) on the raw arrays [`Motor3::to_mat4`](crate::Motor3::to_mat4)/[`crate::Projection`]
/// already return. Lives here, not in `rendering-domain`: it started as a
/// `rendering-domain`-private helper (composing `Camera`'s view-projection
/// matrix), with a note on itself that a second consumer needing generic
/// mat4 multiply was the signal to move it down — a windowed rendering
/// example composing a per-object model matrix with `Camera`'s
/// view-projection is exactly that second consumer. `numeric` (the
/// other candidate) turned out not to fit: it's scalar-only (`Scalar`/
/// `Fixed`, no matrix machinery at all), while `to_mat4` and `Projection`
/// already established `gac` as where this workspace's
/// classical-matrix graphics bridging math lives.
pub fn mat4_mul(lhs: [[Scalar; 4]; 4], rhs: [[Scalar; 4]; 4]) -> [[Scalar; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for (col, rhs_col) in rhs.iter().enumerate() {
        for row in 0..4 {
            out[col][row] = (0..4).map(|k| lhs[k][row] * rhs_col[k]).sum();
        }
    }
    out
}
