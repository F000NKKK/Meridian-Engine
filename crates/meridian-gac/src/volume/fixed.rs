//! Fixed-flavor aliases of the volume types.

use crate::flavor::fixed::FixedFlavor;

/// An axis-aligned box with fixed-point coordinates.
pub type FixedAabb = super::Aabb<FixedFlavor>;
/// A sphere with fixed-point coordinates.
pub type FixedSphere = super::Sphere<FixedFlavor>;
/// An oriented box with fixed-point coordinates.
pub type FixedObb = super::Obb<FixedFlavor>;
/// A cone with fixed-point coordinates.
pub type FixedCone = super::Cone<FixedFlavor>;
/// A plane with fixed-point coefficients.
pub type FixedPlane = super::Plane<FixedFlavor>;
/// A convex volume with fixed-point coordinates.
pub type FixedConvexVolume = super::ConvexVolume<FixedFlavor>;
