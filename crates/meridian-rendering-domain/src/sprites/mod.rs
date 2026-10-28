//! The 2D sprite path, domain side: batches a [`Scene2d`](crate::scene::Scene2d)
//! (or any caller-built quads) into [`SpriteBatch`]es. Uploading and drawing
//! them (premultiplied-alpha blending) is the adapter's `SpriteRenderer`.
//!
//! [`batch_scene2d`] culls to the camera, orders sprites by
//! [`Sprite::layer`](crate::scene::Sprite::layer) (stable, so equal layers keep
//! submission order) and merges runs that share a texture.
//!
//! **Raw path.** Step 1 is one producer of batches, not the only one. A
//! caller can build [`SpriteBatch`]es by hand (any triangle list of
//! [`SpriteVertex`]es in camera units) and go straight to step 2, so custom
//! drawing never has to pretend to be a [`Sprite`](crate::scene::Sprite).
//!
//! **Clipping.** [`batch_scene2d_clipped`] takes [`ClipSpan`]s (a rectangle
//! applied to a range of layers) and tags batches with a [`ClipRect`];
//! the adapter turns it into a scissor rectangle. Hand-built
//! batches set [`SpriteBatch::clip`] directly.
//!
//! Coordinates are the camera's: pixels, origin top-left, `y` down, for
//! [`Camera2d::from_surface_size`](crate::scene::Camera2d::from_surface_size).

mod batch_scene2d;
mod clip_rect;
mod clip_span;
#[cfg(test)]
mod clip_tests;
mod overlay2d;
mod sprite_batch;
mod sprite_quad;
mod sprite_vertex;
mod sprite_view;
#[cfg(test)]
mod tests;

pub use batch_scene2d::{batch_scene2d, batch_scene2d_clipped};
pub use clip_rect::{ClipRect, ScissorRect};
pub use clip_span::ClipSpan;
pub use overlay2d::Overlay2d;
pub use sprite_batch::SpriteBatch;
pub use sprite_vertex::SpriteVertex;
pub use sprite_view::SpriteView;
