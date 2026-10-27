//! The four corners of a [`Sprite`] in camera units.

use meridian_gac::Vec3;

use crate::scene::Sprite;

/// A sprite's quad: corners and texture coordinates, top-left,
/// top-right, bottom-right, bottom-left.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SpriteQuad {
    pub corners: [[f32; 2]; 4],
    pub uvs: [[f32; 2]; 4],
}

impl SpriteQuad {
    /// `frame` places the quad's centre and rotates it about the plane
    /// normal; the rest of the `Motor3` (z, other axes) does not matter.
    pub(super) fn of(sprite: &Sprite) -> Self {
        let [half_w, half_h] = sprite.size.map(|extent| extent / 2.0);
        // Two transforms (centre and the x half-axis): a sprite lives in
        // the scene plane, so the y half-axis is the x one turned a
        // quarter turn and rescaled.
        let centre = sprite.frame.transform_point(Vec3::ZERO);
        let across = sprite.frame.transform_vector(Vec3::new(half_w, 0.0, 0.0));
        let scale = if half_w > 0.0 { half_h / half_w } else { 0.0 };
        let down = Vec3::new(-across.y * scale, across.x * scale, 0.0);
        let corner = |sx: f32, sy: f32| {
            [
                centre.x + sx * across.x + sy * down.x,
                centre.y + sx * across.y + sy * down.y,
            ]
        };
        let corners = [
            corner(-1.0, -1.0),
            corner(1.0, -1.0),
            corner(1.0, 1.0),
            corner(-1.0, 1.0),
        ];
        let [u, v, w, h] = sprite.source_rect.unwrap_or([0.0, 0.0, 1.0, 1.0]);
        let uvs = [[u, v], [u + w, v], [u + w, v + h], [u, v + h]];
        Self { corners, uvs }
    }

    /// `(min, max)` of the corners.
    pub(super) fn bounds(&self) -> ([f32; 2], [f32; 2]) {
        let fold = |pick: fn(f32, f32) -> f32, start: f32| {
            self.corners.iter().fold([start; 2], |acc, c| {
                [pick(acc[0], c[0]), pick(acc[1], c[1])]
            })
        };
        (
            fold(f32::min, f32::INFINITY),
            fold(f32::max, f32::NEG_INFINITY),
        )
    }
}
