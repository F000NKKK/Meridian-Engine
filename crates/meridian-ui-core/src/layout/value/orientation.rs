//! Direction of a stack.

use crate::Size;

/// Main axis of a `StackPanel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Vertical,
    Horizontal,
}

impl Orientation {
    /// The extent of `size` along the stacking direction.
    pub fn main(self, size: Size) -> f32 {
        match self {
            Self::Vertical => size.height,
            Self::Horizontal => size.width,
        }
    }

    /// The extent of `size` across the stacking direction.
    pub fn cross(self, size: Size) -> f32 {
        match self {
            Self::Vertical => size.width,
            Self::Horizontal => size.height,
        }
    }

    /// Builds a size from its main and cross extents.
    pub fn compose(self, main: f32, cross: f32) -> Size {
        match self {
            Self::Vertical => Size::new(cross, main),
            Self::Horizontal => Size::new(main, cross),
        }
    }
}
