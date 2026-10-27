//! A run of vertices drawn with one texture.

use super::{ClipRect, SpriteVertex};
use crate::TextureHandle;

/// Vertices drawn in order with one texture, as a triangle list (three
/// vertices per triangle). Batches draw in slice order, so a later batch
/// covers an earlier one.
#[derive(Debug, Clone)]
pub struct SpriteBatch {
    /// Texture sampled by every vertex of the batch.
    pub texture: TextureHandle,
    /// The triangle list, in draw order.
    pub vertices: Vec<SpriteVertex>,
    /// Where the batch shows; `None` draws unclipped.
    pub clip: Option<ClipRect>,
}

impl SpriteBatch {
    /// An empty batch for `texture`.
    #[must_use]
    pub fn new(texture: TextureHandle) -> Self {
        Self {
            texture,
            vertices: Vec::new(),
            clip: None,
        }
    }

    /// The same batch, clipped to `clip`.
    #[must_use]
    pub fn with_clip(mut self, clip: ClipRect) -> Self {
        self.clip = Some(clip);
        self
    }

    /// Appends one quad as two triangles. `corners` and `uvs` run
    /// top-left, top-right, bottom-right, bottom-left.
    pub fn push_quad(&mut self, corners: [[f32; 2]; 4], uvs: [[f32; 2]; 4], color: [f32; 4]) {
        const TRIANGLES: [usize; 6] = [0, 1, 2, 0, 2, 3];
        self.vertices
            .extend(TRIANGLES.iter().map(|&corner| SpriteVertex {
                position: corners[corner],
                uv: uvs[corner],
                color,
            }));
    }

    /// Number of quads pushed through [`push_quad`](Self::push_quad).
    #[must_use]
    pub fn quad_count(&self) -> usize {
        self.vertices.len() / 6
    }
}
