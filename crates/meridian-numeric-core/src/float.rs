//! The workspace's default (non-deterministic) scalar type — plain `f32`.
//! See [`crate::fixed`] for the deterministic alternative.

/// Workspace scalar type: the single source of truth for float precision.
pub type Scalar = f32;

/// Default tolerance for approximate floating-point comparisons workspace-wide.
pub const EPSILON: Scalar = 1e-5;

/// Approximate equality within [`EPSILON`].
pub fn approx_eq(a: Scalar, b: Scalar) -> bool {
    (a - b).abs() <= EPSILON
}
