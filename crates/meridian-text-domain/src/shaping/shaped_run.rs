//! One line of positioned glyphs.

use super::placed_glyph::PlacedGlyph;

/// One line of text as positioned glyphs, with the metrics to lay it out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShapedRun {
    /// The glyphs, left to right.
    pub glyphs: Vec<PlacedGlyph>,
    /// Total advance in pixels.
    pub width: f32,
    /// Distance from the baseline up to the top of the line, in pixels.
    pub ascent: f32,
    /// Height of a line, gap included, in pixels.
    pub line_height: f32,
}
