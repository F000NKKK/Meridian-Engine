//! WGSL sources for the unlit, lit, textured and emissive-extract pipelines.

mod emissive_extract_shader;
mod lit_shader;
mod textured_shader;
mod unlit_shader;

pub use emissive_extract_shader::EMISSIVE_EXTRACT_SHADER_WGSL;
pub use lit_shader::{MAX_LIGHTS, lit_shader_wgsl, lit_textured_shader_wgsl};
pub use textured_shader::TEXTURED_SHADER_WGSL;
pub use unlit_shader::UNLIT_SHADER_WGSL;
