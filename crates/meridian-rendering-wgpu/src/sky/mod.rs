//! The visible environment: the backdrop of the main scene pass.
//!
//! Design notes: ADR 039.

mod pass;
mod shader;
#[cfg(test)]
mod tests;

pub use pass::SkyPass;
#[cfg(test)]
pub(crate) use pass::uniform_bytes;
pub(crate) use shader::sky_shader_wgsl;
