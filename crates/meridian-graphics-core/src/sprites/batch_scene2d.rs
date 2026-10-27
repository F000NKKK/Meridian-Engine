//! Culling, ordering and merging a [`Scene2d`] into batches.

use super::SpriteBatch;
use super::sprite_quad::SpriteQuad;
use crate::scene::{Camera2d, Scene2d, Sprite};

/// Turns `scene` into draw-ordered batches.
///
/// Sprites fully outside the camera, or with a fully transparent tint, are
/// dropped. The rest are ordered by [`Sprite::layer`] (lower first); equal
/// layers keep their order in `scene.sprites`. Consecutive sprites with the
/// same texture share a batch, so batching never changes what covers what.
#[must_use]
pub fn batch_scene2d(scene: &Scene2d) -> Vec<SpriteBatch> {
    let mut visible: Vec<&Sprite> = scene
        .sprites
        .iter()
        .filter(|sprite| sprite.tint[3] > 0.0)
        .collect();
    visible.sort_by_key(|sprite| sprite.layer);

    let mut batches: Vec<SpriteBatch> = Vec::new();
    for sprite in visible {
        let quad = SpriteQuad::of(sprite);
        if !overlaps_camera(scene.camera, &quad) {
            continue;
        }
        if batches
            .last()
            .is_none_or(|batch| batch.texture != sprite.texture)
        {
            batches.push(SpriteBatch::new(sprite.texture));
        }
        if let Some(batch) = batches.last_mut() {
            batch.push_quad(quad.corners, quad.uvs, sprite.tint);
        }
    }
    batches
}

/// Whether the quad's bounding box touches the camera rectangle.
fn overlaps_camera(camera: Camera2d, quad: &SpriteQuad) -> bool {
    let [left, right, bottom, top] = camera.extents;
    let (min, max) = quad.bounds();
    max[0] >= left.min(right)
        && min[0] <= left.max(right)
        && max[1] >= bottom.min(top)
        && min[1] <= bottom.max(top)
}
