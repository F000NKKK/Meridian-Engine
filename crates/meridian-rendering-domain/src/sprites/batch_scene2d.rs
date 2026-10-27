//! Culling, ordering, clipping and merging a [`Scene2d`] into batches.

use super::sprite_quad::SpriteQuad;
use super::{ClipRect, ClipSpan, SpriteBatch};
use crate::TextureHandle;
use crate::scene::{Camera2d, Scene2d, Sprite};

/// Turns `scene` into draw-ordered, unclipped batches; see
/// [`batch_scene2d_clipped`].
#[must_use]
pub fn batch_scene2d(scene: &Scene2d) -> Vec<SpriteBatch> {
    batch_scene2d_clipped(scene, &[])
}

/// Turns `scene` into draw-ordered batches, clipping by `clips`.
///
/// Sprites fully outside the camera or their clip, or with a fully
/// transparent tint, are dropped. The rest are ordered by
/// [`Sprite::layer`] (lower first); equal layers keep their order in
/// `scene.sprites`. Consecutive sprites with the same texture and clip
/// share a batch, so batching never changes what covers what.
#[must_use]
pub fn batch_scene2d_clipped(scene: &Scene2d, clips: &[ClipSpan]) -> Vec<SpriteBatch> {
    let mut visible: Vec<&Sprite> = scene
        .sprites
        .iter()
        .filter(|sprite| sprite.tint[3] > 0.0)
        .collect();
    visible.sort_by_key(|sprite| sprite.layer);

    let mut batches: Vec<SpriteBatch> = Vec::new();
    for sprite in visible {
        let quad = SpriteQuad::of(sprite);
        let (min, max) = quad.bounds();
        if !overlaps_camera(scene.camera, min, max) {
            continue;
        }
        let Clipping::Show(clip) = clipping_of(sprite.layer, clips) else {
            continue;
        };
        if clip.is_some_and(|rect| !rect.overlaps(min, max)) {
            continue;
        }
        let starts_new = batches
            .last()
            .is_none_or(|last| last.texture != sprite.texture || last.clip != clip);
        if starts_new {
            batches.push(new_batch(sprite.texture, clip));
        }
        if let Some(batch) = batches.last_mut() {
            batch.push_quad(quad.corners, quad.uvs, sprite.tint);
        }
    }
    batches
}

/// What the clip spans do to one layer.
enum Clipping {
    /// Draw, inside the rectangle if there is one.
    Show(Option<ClipRect>),
    /// The spans covering the layer do not overlap: nothing shows.
    Hidden,
}

/// The intersection of every span covering `layer`.
fn clipping_of(layer: i32, clips: &[ClipSpan]) -> Clipping {
    let mut clip: Option<ClipRect> = None;
    for span in clips.iter().filter(|span| span.layers.contains(&layer)) {
        clip = match clip.map_or(Some(span.rect), |current| current.intersect(span.rect)) {
            Some(rect) => Some(rect),
            None => return Clipping::Hidden,
        };
    }
    Clipping::Show(clip)
}

fn new_batch(texture: TextureHandle, clip: Option<ClipRect>) -> SpriteBatch {
    let batch = SpriteBatch::new(texture);
    clip.map_or(batch.clone(), |rect| batch.with_clip(rect))
}

/// Whether the box `min..max` touches the camera rectangle.
fn overlaps_camera(camera: Camera2d, min: [f32; 2], max: [f32; 2]) -> bool {
    let [left, right, bottom, top] = camera.extents;
    max[0] >= left.min(right)
        && min[0] <= left.max(right)
        && max[1] >= bottom.min(top)
        && min[1] <= bottom.max(top)
}
