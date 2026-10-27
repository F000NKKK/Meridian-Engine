//! A glyph's shape.

use crate::{Contour, Point, Segment};

/// A glyph's shape: contours in font units, filled with the non-zero
/// winding rule. Public so any source of vector shapes (an icon set, a
/// game's own art) can use the rasterizer without a font file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Outline {
    /// The closed loops.
    pub contours: Vec<Contour>,
}

impl Outline {
    /// Whether the outline draws nothing (a space, for instance).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.contours.is_empty()
    }

    /// The tightest box around every point, control points included, as
    /// `(min, max)`; `None` for an empty outline.
    #[must_use]
    pub fn bounds(&self) -> Option<(Point, Point)> {
        let mut points = self.contours.iter().flat_map(contour_points);
        let first = points.next()?;
        Some(points.fold((first, first), |(min, max), p| {
            (
                Point::new(min.x.min(p.x), min.y.min(p.y)),
                Point::new(max.x.max(p.x), max.y.max(p.y)),
            )
        }))
    }

    /// Every point moved by `(dx, dy)`.
    pub(crate) fn translate(&mut self, dx: f32, dy: f32) {
        self.map_points(|p| Point::new(p.x + dx, p.y + dy));
    }

    /// Every point through `matrix` = `[xx, xy, yx, yy]`.
    pub(crate) fn transform(&mut self, matrix: [f32; 4]) {
        let [xx, xy, yx, yy] = matrix;
        self.map_points(|p| Point::new(p.x * xx + p.y * yx, p.x * xy + p.y * yy));
    }

    fn map_points(&mut self, map: impl Fn(Point) -> Point) {
        for contour in &mut self.contours {
            contour.start = map(contour.start);
            for segment in &mut contour.segments {
                *segment = match *segment {
                    Segment::Line { to } => Segment::Line { to: map(to) },
                    Segment::Quad { control, to } => Segment::Quad {
                        control: map(control),
                        to: map(to),
                    },
                };
            }
        }
    }
}

fn contour_points(contour: &Contour) -> impl Iterator<Item = Point> + '_ {
    std::iter::once(contour.start).chain(contour.segments.iter().flat_map(|segment| {
        let (control, to) = match *segment {
            Segment::Line { to } => (None, to),
            Segment::Quad { control, to } => (Some(control), to),
        };
        control.into_iter().chain(std::iter::once(to))
    }))
}
