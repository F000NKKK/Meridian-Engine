//! A point of a TrueType contour.

use crate::Point;

/// A point of a TrueType contour, on or off the curve.
#[derive(Debug, Clone, Copy)]
pub(super) struct RawPoint {
    pub(super) at: Point,
    pub(super) on_curve: bool,
}
