//! Width and height.

use super::Thickness;

/// A 2D extent. Unbounded axes are `f32::INFINITY`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    /// "No constraint" on both axes, used while measuring.
    pub const UNBOUNDED: Self = Self::new(f32::INFINITY, f32::INFINITY);

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Shrinks by `inset`, never below zero.
    pub fn deflate(self, inset: Thickness) -> Self {
        Self::new(
            (self.width - inset.horizontal()).max(0.0),
            (self.height - inset.vertical()).max(0.0),
        )
    }

    /// Grows by `inset`.
    pub fn inflate(self, inset: Thickness) -> Self {
        Self::new(
            self.width + inset.horizontal(),
            self.height + inset.vertical(),
        )
    }
}
