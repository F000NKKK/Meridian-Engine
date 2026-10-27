//! Shadow mapping, domain side: the light and the camera that fits its volume.

mod shadow;
mod shadow_light;

pub use shadow::{SHADOW_VOLUME_HALF_EXTENT, directional_shadow_camera};
pub use shadow_light::ShadowLight;
