//! One closed loop of an outline.

use crate::{Point, Segment};

/// A closed loop: it starts at `start`, follows `segments`, and closes with
/// a straight line back to `start` when the last segment ends elsewhere.
#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    /// Where the loop begins.
    pub start: Point,
    /// The pieces, in drawing order.
    pub segments: Vec<Segment>,
}
