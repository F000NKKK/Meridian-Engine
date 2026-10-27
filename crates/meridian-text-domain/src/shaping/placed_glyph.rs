//! A glyph and where its pen position is.

use crate::GlyphId;

/// A glyph with the pen position it is drawn from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedGlyph {
    /// Which glyph.
    pub glyph: GlyphId,
    /// Pen position along the line, in pixels from the start of the run.
    pub x: f32,
}
