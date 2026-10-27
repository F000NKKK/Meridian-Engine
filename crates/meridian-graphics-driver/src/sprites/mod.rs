//! The sprite recording path: uploads a domain `SpriteBatch` list and draws it
//! with premultiplied-alpha blending (the batching itself is `graphics-core`).
//!
//! [`SpriteRenderer::prepare`] uploads the batches before any render pass is
//! open; [`SpriteRenderer::draw`] records them into the caller's pass, turning
//! a batch's `ClipRect` into a scissor rectangle.

mod prepared_sprites;
mod sprite_encoding;
mod sprite_renderer;
mod sprite_run;
mod sprite_shader_table;
#[cfg(test)]
mod tests;

pub use prepared_sprites::PreparedSprites;
pub use sprite_encoding::SPRITE_VERTEX_BYTES;
pub use sprite_renderer::SpriteRenderer;
pub use sprite_shader_table::SPRITE_SHADER_WGSL;
