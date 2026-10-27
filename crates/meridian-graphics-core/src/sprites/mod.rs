//! The 2D sprite path: batches a [`Scene2d`](crate::scene::Scene2d) (or any
//! caller-built quads) and draws it with premultiplied-alpha blending.
//!
//! Three steps, the same shape as the 3D path in [`crate::submission`]:
//!
//! 1. **Batch** (CPU only, no GPU): [`batch_scene2d`] culls to the camera,
//!    orders sprites by [`Sprite::layer`](crate::scene::Sprite::layer)
//!    (stable, so equal layers keep submission order) and merges runs that
//!    share a texture into [`SpriteBatch`]es.
//! 2. **Prepare**: [`SpriteRenderer::prepare`] uploads the batches before
//!    any render pass is open.
//! 3. **Draw**: [`SpriteRenderer::draw`] records them into the caller's pass.
//!
//! **Raw path.** Step 1 is one producer of batches, not the only one. A
//! caller can build [`SpriteBatch`]es by hand (any triangle list of
//! [`SpriteVertex`]es in camera units) and go straight to step 2, so custom
//! drawing never has to pretend to be a [`Sprite`](crate::scene::Sprite).
//!
//! **Clipping.** [`batch_scene2d_clipped`] takes [`ClipSpan`]s (a rectangle
//! applied to a range of layers) and tags batches with a [`ClipRect`];
//! [`SpriteRenderer::draw`] turns it into a scissor rectangle. Hand-built
//! batches set [`SpriteBatch::clip`] directly.
//!
//! Coordinates are the camera's: pixels, origin top-left, `y` down, for
//! [`Camera2d::from_surface_size`](crate::scene::Camera2d::from_surface_size).

mod batch_scene2d;
mod clip_rect;
mod clip_span;
#[cfg(test)]
mod clip_tests;
mod prepared_sprites;
mod sprite_batch;
mod sprite_quad;
mod sprite_renderer;
mod sprite_run;
mod sprite_shader_table;
mod sprite_vertex;
mod sprite_view;
#[cfg(test)]
mod tests;

pub use batch_scene2d::{batch_scene2d, batch_scene2d_clipped};
pub use clip_rect::{ClipRect, ScissorRect};
pub use clip_span::ClipSpan;
pub use prepared_sprites::PreparedSprites;
pub use sprite_batch::SpriteBatch;
pub use sprite_renderer::SpriteRenderer;
pub use sprite_shader_table::SPRITE_SHADER_WGSL;
pub use sprite_vertex::{SPRITE_VERTEX_BYTES, SpriteVertex};
pub use sprite_view::SpriteView;
