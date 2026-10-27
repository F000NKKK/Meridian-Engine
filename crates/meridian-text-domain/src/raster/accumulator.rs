//! Exact-area coverage accumulation (the signed-area trick).
//!
//! Each line adds, to the cells it crosses, the signed area it sweeps to
//! the right of itself; a running sum along every row then turns those
//! into the coverage of each pixel. Rows are `stride = width + 1` cells so
//! an edge at the right border has somewhere to write.

use crate::Point;

#[derive(Debug)]
pub(super) struct Accumulator {
    width: usize,
    height: usize,
    stride: usize,
    cells: Vec<f32>,
}

impl Accumulator {
    pub(super) fn new(width: usize, height: usize) -> Self {
        let stride = width + 1;
        Self {
            width,
            height,
            stride,
            cells: vec![0.0; stride * height + 2],
        }
    }

    /// Adds the line `from -> to`, in pixel space with `y` down.
    #[expect(
        clippy::float_cmp,
        reason = "only exactly horizontal lines add no area"
    )]
    pub(super) fn line(&mut self, from: Point, to: Point) {
        if from.y == to.y {
            return;
        }
        let (direction, top, bottom) = if from.y < to.y {
            (1.0, from, to)
        } else {
            (-1.0, to, from)
        };
        let slope = (bottom.x - top.x) / (bottom.y - top.y);
        let start = top.y.max(0.0);
        let mut x = top.x + (start - top.y) * slope;
        let end = index(bottom.y.ceil()).min(self.height);
        for row in index(start)..end {
            let row_top = as_f32(row).max(top.y);
            let row_bottom = as_f32(row + 1).min(bottom.y);
            let dy = row_bottom - row_top;
            let next = x + slope * dy;
            self.cross(row, x, next, dy * direction);
            x = next;
        }
    }

    /// Coverage per pixel, rows top to bottom.
    pub(super) fn coverage(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.width * self.height);
        let mut sum = 0.0_f32;
        for row in self.cells.chunks(self.stride).take(self.height) {
            for (column, cell) in row.iter().enumerate() {
                sum += cell;
                if column < self.width {
                    out.push(to_byte(sum.abs().min(1.0)));
                }
            }
        }
        out
    }

    /// One row's share of a line: it moves from `xa` to `xb` while sweeping
    /// `area` (signed height) of the row.
    fn cross(&mut self, row: usize, xa: f32, xb: f32, area: f32) {
        let limit = as_f32(self.width);
        let low = xa.min(xb).clamp(0.0, limit);
        let high = xa.max(xb).clamp(0.0, limit);
        let base = row * self.stride;
        let first = index(low.floor());
        let last = index(high.ceil());
        if last <= first + 1 {
            let mid = 0.5 * (low + high) - low.floor();
            self.cells[base + first] += area - area * mid;
            self.cells[base + first + 1] += area * mid;
        } else {
            self.spread(base, (low, high), area);
        }
    }

    /// A line that crosses several pixels of one row.
    fn spread(&mut self, base: usize, (low, high): (f32, f32), area: f32) {
        let first = index(low.floor());
        let last = index(high.ceil());
        let inverse = 1.0 / (high - low);
        let low_fraction = low - low.floor();
        let first_share = 0.5 * inverse * (1.0 - low_fraction) * (1.0 - low_fraction);
        let high_fraction = high - high.ceil() + 1.0;
        let last_share = 0.5 * inverse * high_fraction * high_fraction;
        self.cells[base + first] += area * first_share;
        if last == first + 2 {
            self.cells[base + first + 1] += area * (1.0 - first_share - last_share);
        } else {
            let second_share = inverse * (1.5 - low_fraction);
            self.cells[base + first + 1] += area * (second_share - first_share);
            for column in first + 2..last - 1 {
                self.cells[base + column] += area * inverse;
            }
            let covered = second_share + as_f32(last - first - 3) * inverse;
            self.cells[base + last - 1] += area * (1.0 - covered - last_share);
        }
        self.cells[base + last] += area * last_share;
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "rows and columns are below 4097, exact in f32"
)]
const fn as_f32(value: usize) -> f32 {
    value as f32
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "callers pass non-negative values already bounded by the bitmap size"
)]
fn index(value: f32) -> usize {
    value.max(0.0) as usize
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is within 0..=1 and scaled to 0..=255"
)]
fn to_byte(coverage: f32) -> u8 {
    (coverage * 255.0 + 0.5) as u8
}
