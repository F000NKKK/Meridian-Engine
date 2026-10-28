//! WGSL sources for the unlit, lit, textured and emissive-extract pipelines.

mod emissive_extract_shader;
mod environment_shader;
mod lit_shader;
mod shadow_depth_shader;
mod shadow_lookup_shader;
mod textured_shader;
mod unlit_shader;

pub use emissive_extract_shader::EMISSIVE_EXTRACT_SHADER_WGSL;
pub(crate) use emissive_extract_shader::EMISSIVE_MAPPED_EXTRACT_SHADER_WGSL;
pub use lit_shader::MAX_LIGHTS;
pub(crate) use lit_shader::{lit_shader_wgsl, lit_textured_shader_wgsl};
pub use shadow_depth_shader::SHADOW_DEPTH_SHADER_WGSL;
pub use textured_shader::TEXTURED_SHADER_WGSL;
pub use unlit_shader::UNLIT_SHADER_WGSL;
