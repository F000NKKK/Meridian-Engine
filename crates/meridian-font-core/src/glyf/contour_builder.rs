//! Turns TrueType's on/off-curve point loops into contours.

use crate::{Contour, Point, Segment};

/// A point of a TrueType contour.
#[derive(Debug, Clone, Copy)]
pub(super) struct RawPoint {
    pub(super) at: Point,
    pub(super) on_curve: bool,
}

/// One contour from its points. Consecutive off-curve points imply an
/// on-curve point halfway between them; a loop with no on-curve point
/// starts halfway between its first and last.
pub(super) fn build(points: &[RawPoint]) -> Option<Contour> {
    let (start, rest) = split_start(points)?;
    let mut segments = Vec::with_capacity(rest.len() + 1);
    let mut pending: Option<Point> = None;
    for point in rest {
        match (point.on_curve, pending.take()) {
            (true, None) => segments.push(Segment::Line { to: point.at }),
            (true, Some(control)) => segments.push(Segment::Quad {
                control,
                to: point.at,
            }),
            (false, None) => pending = Some(point.at),
            (false, Some(control)) => {
                segments.push(Segment::Quad {
                    control,
                    to: control.midpoint(point.at),
                });
                pending = Some(point.at);
            }
        }
    }
    if let Some(control) = pending {
        segments.push(Segment::Quad { control, to: start });
    }
    Some(Contour { start, segments })
}

/// The starting point and the points to walk after it, in order.
fn split_start(points: &[RawPoint]) -> Option<(Point, Vec<RawPoint>)> {
    let first = *points.first()?;
    let last = *points.last()?;
    if first.on_curve {
        return Some((first.at, points[1..].to_vec()));
    }
    if last.on_curve {
        return Some((last.at, points[..points.len() - 1].to_vec()));
    }
    Some((first.at.midpoint(last.at), points.to_vec()))
}
