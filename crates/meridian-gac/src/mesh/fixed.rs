//! Fixed-flavor mesh alias and icosphere.

use crate::flavor::fixed::FixedFlavor;

/// An indexed polygon mesh with fixed-point vertex coordinates.
pub type FixedMesh = super::Mesh<FixedFlavor>;

/// See [`super::icosphere`].
pub fn fixed_icosphere(subdivisions: u32) -> FixedMesh {
    super::icosphere::<FixedFlavor>(subdivisions)
}
