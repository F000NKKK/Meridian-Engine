//! Fixed-flavor aliases of the volume types.

use crate::flavor::fixed::FixedFlavor;

pub type FixedAabb = super::Aabb<FixedFlavor>;
pub type FixedSphere = super::Sphere<FixedFlavor>;
pub type FixedObb = super::Obb<FixedFlavor>;
pub type FixedCone = super::Cone<FixedFlavor>;
pub type FixedPlane = super::Plane<FixedFlavor>;
pub type FixedConvexVolume = super::ConvexVolume<FixedFlavor>;
