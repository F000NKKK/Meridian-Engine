//! Fixed-point-flavor soft bodies: an opt-in, bit-reproducible deformable-body
//! path for lockstep networking and replay. `Fixed::sqrt` (used for every
//! spring's current length) is exact and deterministic on the CPU, so this
//! needed nothing beyond aliasing the generic engine.

use super::ball::{self, SoftBodyMaterial as GenericMaterial};
use meridian_gac_core::{FixedFlavor, FixedVec3};
use meridian_numeric_core::Fixed;

/// A spring of the soft body, in this flavor.
pub type FixedSpring = super::spring::Spring<FixedFlavor>;
/// A deformable body, in this flavor.
pub type FixedSoftBody = super::mass_spring::SoftBody<FixedFlavor>;
/// The soft-body time stepper, in this flavor.
pub type FixedSoftBodyIntegrator = super::time_step::SoftBodyIntegrator<FixedFlavor>;
/// Spring tuning, in this flavor.
pub type FixedSpringTuning = ball::SpringTuning<FixedFlavor>;
/// Soft-ball material parameters, in this flavor.
pub type FixedSoftBodyMaterial = GenericMaterial<FixedFlavor>;

/// See [`ball::icosphere_soft_body`].
#[must_use]
pub fn fixed_icosphere_soft_body(
    center: FixedVec3,
    radius: Fixed,
    subdivisions: u32,
    material: FixedSoftBodyMaterial,
) -> FixedSoftBody {
    ball::icosphere_soft_body::<FixedFlavor>(center, radius, subdivisions, material)
}
