//! Bloom: a glow around bright (`Material::emissive`) surfaces — the
//! first piece of docs/graphics-design.md's "Post Processing" scaffold.
//! Built entirely on primitives [`crate::submission`] and
//! this crate already expose: an emissive-only redraw of the same
//! [`crate::submission::DrawBuffers`] the main pass baked, a separable
//! Gaussian blur, and an additive composite back onto the main render
//! target.
//!
//! Design notes: docs/notes/meridian-rendering-domain.md (`bloom`).

mod blur_direction;
mod input;
mod pass;
mod shaders;
#[cfg(test)]
mod tests;

pub(crate) use blur_direction::BlurDirection;
pub use input::BloomInput;
pub use pass::BloomPass;
#[cfg(test)]
pub(crate) use pass::blur_uniform_bytes;
pub(crate) use shaders::{blur_shader_wgsl, composite_shader_wgsl};
