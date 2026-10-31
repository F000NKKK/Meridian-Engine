//! Float-flavor mesh alias and icosphere.

use crate::flavor::float::FloatFlavor;

/// An indexed polygon mesh with floating-point vertex coordinates.
pub type Mesh = super::Mesh<FloatFlavor>;

/// See [`super::icosphere`].
pub fn icosphere(subdivisions: u32) -> Mesh {
    super::icosphere::<FloatFlavor>(subdivisions)
}
