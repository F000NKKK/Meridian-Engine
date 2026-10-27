//! Filling an outline into a bitmap.

use super::accumulator::Accumulator;
use super::bitmap::Bitmap;
use super::flatten;
use crate::{Contour, FontError, Outline, Point, Segment};

/// Largest bitmap side in pixels; a guard against absurd scales.
const MAX_SIDE: f32 = 4096.0;

/// Fills `outline` (font units, `y` up) at `scale` pixels per unit, shifted
/// by `offset` pixels, with the non-zero winding rule and area-exact
/// anti-aliasing. An empty outline gives an empty bitmap.
///
/// # Errors
///
/// [`FontError::BitmapTooLarge`] when the result would exceed 4096 pixels
/// on a side, or the scale is not a finite number.
pub fn rasterize(outline: &Outline, scale: f32, offset: Point) -> Result<Bitmap, FontError> {
    let Some((min, max)) = outline.bounds() else {
        return Ok(Bitmap::default());
    };
    let to_pixels = |p: Point| Point::new(p.x * scale + offset.x, p.y * scale + offset.y);
    let (min, max) = (to_pixels(min), to_pixels(max));
    let (left, right) = (min.x.floor(), max.x.ceil());
    let (bottom, top) = (min.y.floor(), max.y.ceil());
    let (width, height) = (right - left, top - bottom);
    if !(width.is_finite() && height.is_finite()) || width > MAX_SIDE || height > MAX_SIDE {
        return Err(FontError::BitmapTooLarge);
    }
    let to_cell = |p: Point| {
        let p = to_pixels(p);
        Point::new(p.x - left, top - p.y)
    };
    let (columns, rows) = (whole(width), whole(height));
    let mut accumulator = Accumulator::new(columns, rows);
    for contour in &outline.contours {
        trace(&mut accumulator, contour, to_cell);
    }
    Ok(Bitmap {
        width: u32::try_from(columns).map_err(|_| FontError::BitmapTooLarge)?,
        height: u32::try_from(rows).map_err(|_| FontError::BitmapTooLarge)?,
        left: whole_signed(left),
        top: whole_signed(top),
        coverage: accumulator.coverage(),
    })
}

/// Walks a contour, feeding its lines to the accumulator, and closes it.
fn trace(accumulator: &mut Accumulator, contour: &Contour, to_cell: impl Fn(Point) -> Point) {
    let start = to_cell(contour.start);
    let mut at = start;
    for segment in &contour.segments {
        match *segment {
            Segment::Line { to } => {
                let to = to_cell(to);
                accumulator.line(at, to);
                at = to;
            }
            Segment::Quad { control, to } => {
                let (control, to) = (to_cell(control), to_cell(to));
                for next in flatten::quad(at, control, to) {
                    accumulator.line(at, next);
                    at = next;
                }
            }
        }
    }
    accumulator.line(at, start);
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bounded by MAX_SIDE and non-negative"
)]
const fn whole(value: f32) -> usize {
    value as usize
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded by MAX_SIDE and the offset"
)]
const fn whole_signed(value: f32) -> i32 {
    value as i32
}
