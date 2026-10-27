//! Convex volume: an intersection of half-spaces.

use super::{Plane, Shape};
use crate::flavor::GaFlavor;

/// An arbitrary convex region defined as an intersection of half-spaces —
/// the generalization of a camera frustum (always exactly 6 planes) to any
/// number of planes, so it can describe *any* convex bounding volume, not
/// just a camera's view volume. Works against any [`Shape`] via
/// [`Plane::contains`], not a hardcoded shape list.
#[derive(Debug, Clone)]
pub struct ConvexVolume<F: GaFlavor> {
    pub planes: Vec<Plane<F>>,
}

impl<F: GaFlavor> Default for ConvexVolume<F> {
    fn default() -> Self {
        Self { planes: Vec::new() }
    }
}

impl<F: GaFlavor> ConvexVolume<F> {
    pub fn new(planes: Vec<Plane<F>>) -> Self {
        Self { planes }
    }

    /// Conservative test: `false` means `shape` is fully outside at least
    /// one plane (definitely not overlapping this volume); `true` means
    /// it's on the inside half-space of every plane (overlapping, or a
    /// false positive near a corner — the standard convex-volume trade-off,
    /// cheaper than exact separation).
    pub fn intersects<S: Shape<F>>(&self, shape: &S) -> bool {
        self.planes.iter().all(|plane| plane.contains(shape))
    }
}
