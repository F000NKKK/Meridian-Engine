//! Tone mapping: the pass that turns the HDR scene into the swapchain image.
//!
//! Design notes: ADR 034.

mod pass;
mod shader;
#[cfg(test)]
mod tests;

pub use pass::TonemapPass;
#[cfg(test)]
pub(crate) use pass::uniform_bytes;
pub(crate) use shader::tonemap_shader_wgsl;
