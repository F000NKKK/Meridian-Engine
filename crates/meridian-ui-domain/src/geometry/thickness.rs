//! Edge insets (margin, padding).

/// Per-edge inset, as WPF's `Thickness`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Thickness {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Thickness {
    pub const ZERO: Self = Self::uniform(0.0);

    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    /// Total inset on the horizontal axis.
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    /// Total inset on the vertical axis.
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}
