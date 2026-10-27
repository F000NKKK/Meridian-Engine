//! An axis-aligned rectangle.

use super::{Point, Size, Thickness};

/// A rectangle: origin plus size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub const fn from_size(size: Size) -> Self {
        Self {
            origin: Point::new(0.0, 0.0),
            size,
        }
    }

    /// Half-open containment: the left/top edges are inside, right/bottom
    /// are not, so a zero-sized rectangle contains nothing.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.y >= self.origin.y
            && point.x < self.origin.x + self.size.width
            && point.y < self.origin.y + self.size.height
    }

    /// Shrinks by `inset` on every edge, never below zero size.
    pub fn deflate(self, inset: Thickness) -> Self {
        Self {
            origin: Point::new(self.origin.x + inset.left, self.origin.y + inset.top),
            size: self.size.deflate(inset),
        }
    }
}
