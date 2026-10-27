//! Float-flavor (`f32`) soft bodies, the default deformable-body path.

use meridian_gac::{FloatFlavor, Vec3};

use super::ball::{self, SoftBodyMaterial as GenericMaterial};

/// A spring of the soft body, in this flavor.
pub type Spring = super::spring::Spring<FloatFlavor>;
/// A deformable body, in this flavor.
pub type SoftBody = super::mass_spring::SoftBody<FloatFlavor>;
/// The soft-body time stepper, in this flavor.
pub type SoftBodyIntegrator = super::time_step::SoftBodyIntegrator<FloatFlavor>;
/// Spring tuning, in this flavor.
pub type SpringTuning = ball::SpringTuning<FloatFlavor>;
/// Soft-ball material parameters, in this flavor.
pub type SoftBodyMaterial = GenericMaterial<FloatFlavor>;

/// See [`ball::icosphere_soft_body`].
#[must_use]
pub fn icosphere_soft_body(
    center: Vec3,
    radius: f32,
    subdivisions: u32,
    material: SoftBodyMaterial,
) -> SoftBody {
    ball::icosphere_soft_body::<FloatFlavor>(center, radius, subdivisions, material)
}
