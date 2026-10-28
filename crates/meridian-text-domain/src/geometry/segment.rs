//! One piece of a contour.

use crate::Point;

/// A piece of a contour, ending at `to`; it starts where the previous one
/// ended.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Segment {
    /// A straight line.
    Line {
        /// The end point.
        to: Point,
    },
    /// A quadratic Bezier curve.
    Quad {
        /// The off-curve control point.
        control: Point,
        /// The end point.
        to: Point,
    },
}

impl Segment {
    /// Where the segment ends.
    #[must_use]
    pub const fn end(self) -> Point {
        match self {
            Self::Line { to } | Self::Quad { to, .. } => to,
        }
    }
}
