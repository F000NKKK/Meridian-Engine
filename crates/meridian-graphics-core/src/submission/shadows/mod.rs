//! Shadow mapping: light, camera and depth-only pass.

mod shadow;
mod shadow_depth_pass;
mod shadow_light;

pub use shadow::{SHADOW_DEPTH_SHADER_WGSL, SHADOW_VOLUME_HALF_EXTENT, directional_shadow_camera};
pub(crate) use shadow_depth_pass::ShadowDepthPass;
pub use shadow_light::ShadowLight;
