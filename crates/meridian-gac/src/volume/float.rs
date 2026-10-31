//! Float-flavor aliases of the volume types.

use crate::flavor::float::FloatFlavor;

/// An axis-aligned box with floating-point coordinates.
pub type Aabb = super::Aabb<FloatFlavor>;
/// A sphere with floating-point coordinates.
pub type Sphere = super::Sphere<FloatFlavor>;
/// An oriented box with floating-point coordinates.
pub type Obb = super::Obb<FloatFlavor>;
/// A cone with floating-point coordinates.
pub type Cone = super::Cone<FloatFlavor>;
/// A plane with floating-point coefficients.
pub type Plane = super::Plane<FloatFlavor>;
/// A convex volume with floating-point coordinates.
pub type ConvexVolume = super::ConvexVolume<FloatFlavor>;
