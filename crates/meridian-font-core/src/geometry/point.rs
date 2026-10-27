//! A position in font units.

/// A position in font units: `y` grows upwards, the origin is the glyph
/// origin on the baseline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal position.
    pub x: f32,
    /// Vertical position, up.
    pub y: f32,
}

impl Point {
    /// A point at `(x, y)`.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Halfway between two points.
    #[must_use]
    pub fn midpoint(self, other: Self) -> Self {
        Self::new(
            f32::midpoint(self.x, other.x),
            f32::midpoint(self.y, other.y),
        )
    }
}
