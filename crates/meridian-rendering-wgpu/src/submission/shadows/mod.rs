//! The depth-only pass that fills the shadow map.

mod cascade_casters;
mod shadow_depth_pass;

pub(crate) use cascade_casters::CascadeCasters;
pub(crate) use shadow_depth_pass::ShadowDepthPass;
