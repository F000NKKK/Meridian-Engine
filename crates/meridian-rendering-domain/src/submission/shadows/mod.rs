//! Shadow mapping, domain side: the light and the camera that fits its volume.

mod cascade_fit;
mod cascade_frustum;
mod point_shadow;
#[cfg(test)]
mod point_shadow_tests;
mod shadow;
mod shadow_cascades;
mod shadow_light;
#[cfg(test)]
mod tests;

pub use point_shadow::{MAX_POINT_SHADOWS, POINT_SHADOW_FACES, POINT_SHADOW_NEAR, PointShadow};
pub use shadow::{SHADOW_VOLUME_HALF_EXTENT, directional_shadow_camera};
pub use shadow_cascades::{MAX_SHADOW_DISTANCE, SHADOW_CASCADES, ShadowCascades};
pub use shadow_light::ShadowLight;
