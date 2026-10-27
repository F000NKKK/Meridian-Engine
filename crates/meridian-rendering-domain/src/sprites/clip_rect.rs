//! A rectangle that clips sprites, in camera units.

use crate::scene::Camera2d;

/// Float noise below this many pixels does not widen a scissor.
const SNAP: f32 = 1e-3;

/// A pixel rectangle on the render target, origin top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScissorRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// An axis-aligned rectangle in the camera's units. Sprites drawn under
/// it show only inside it (a scroll viewer's content, a clipped panel).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipRect {
    /// The corner with the smallest `x` and `y`.
    pub origin: [f32; 2],
    /// Width and height, both `>= 0`.
    pub size: [f32; 2],
}

impl ClipRect {
    /// The part of both rectangles, or `None` if they do not overlap.
    #[must_use]
    pub fn intersect(self, other: Self) -> Option<Self> {
        let min = [
            self.origin[0].max(other.origin[0]),
            self.origin[1].max(other.origin[1]),
        ];
        let max = [
            (self.origin[0] + self.size[0]).min(other.origin[0] + other.size[0]),
            (self.origin[1] + self.size[1]).min(other.origin[1] + other.size[1]),
        ];
        (max[0] > min[0] && max[1] > min[1]).then(|| Self {
            origin: min,
            size: [max[0] - min[0], max[1] - min[1]],
        })
    }

    /// Whether the box `min..max` touches the rectangle.
    #[must_use]
    pub fn overlaps(self, min: [f32; 2], max: [f32; 2]) -> bool {
        max[0] >= self.origin[0]
            && min[0] <= self.origin[0] + self.size[0]
            && max[1] >= self.origin[1]
            && min[1] <= self.origin[1] + self.size[1]
    }

    /// The pixels of a `viewport`-sized target this rectangle covers under
    /// `camera`, rounded outward (ignoring float noise) and clamped to the target; `None` when
    /// nothing is left.
    #[must_use]
    pub fn to_scissor(self, camera: Camera2d, viewport: [u32; 2]) -> Option<ScissorRect> {
        let [left, right, bottom, top] = camera.extents;
        let [width, height] = viewport.map(pixel_count);
        let to_pixels = |x: f32, y: f32| {
            [
                (x - left) / (right - left) * width,
                (y - top) / (bottom - top) * height,
            ]
        };
        let a = to_pixels(self.origin[0], self.origin[1]);
        let b = to_pixels(self.origin[0] + self.size[0], self.origin[1] + self.size[1]);
        let x0 = to_pixel((a[0].min(b[0]) + SNAP).floor(), viewport[0]);
        let x1 = to_pixel((a[0].max(b[0]) - SNAP).ceil(), viewport[0]);
        let y0 = to_pixel((a[1].min(b[1]) + SNAP).floor(), viewport[1]);
        let y1 = to_pixel((a[1].max(b[1]) - SNAP).ceil(), viewport[1]);
        (x1 > x0 && y1 > y0).then_some(ScissorRect {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        })
    }
}

/// A pixel count as a float.
#[expect(
    clippy::cast_precision_loss,
    reason = "target sizes are far below 2^24"
)]
fn pixel_count(pixels: u32) -> f32 {
    pixels as f32
}

/// `value` clamped to `0..=limit` pixels.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 0..=limit first, and limit fits a u32"
)]
fn to_pixel(value: f32, limit: u32) -> u32 {
    value.clamp(0.0, pixel_count(limit)) as u32
}
