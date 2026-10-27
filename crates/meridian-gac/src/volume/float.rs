//! Float-flavor aliases of the volume types.

use crate::flavor::float::FloatFlavor;

pub type Aabb = super::Aabb<FloatFlavor>;
pub type Sphere = super::Sphere<FloatFlavor>;
pub type Obb = super::Obb<FloatFlavor>;
pub type Cone = super::Cone<FloatFlavor>;
pub type Plane = super::Plane<FloatFlavor>;
pub type ConvexVolume = super::ConvexVolume<FloatFlavor>;
