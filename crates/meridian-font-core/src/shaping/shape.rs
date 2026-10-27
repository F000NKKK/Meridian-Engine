//! The built-in shaper: left to right, one glyph per character, kerning.

use super::placed_glyph::PlacedGlyph;
use super::shaped_run::ShapedRun;
use crate::{FontFace, GlyphId};

/// Shapes `text` as one line at `pixel_size` pixels per em: each character
/// becomes its glyph (the face's missing glyph when it has none), advanced
/// by `hmtx` and adjusted by `kern`. Control characters, `\n` included,
/// draw nothing. Ligatures, bidirectional text and complex scripts are not
/// handled; a game with those needs its own shaper over the same types.
#[must_use]
pub fn shape(face: &FontFace, text: &str, pixel_size: f32) -> ShapedRun {
    let scale = pixel_size / f32::from(face.units_per_em());
    let mut glyphs = Vec::with_capacity(text.len());
    let mut pen = 0.0;
    let mut previous: Option<GlyphId> = None;
    for c in text.chars().filter(|c| !c.is_control()) {
        let glyph = face.glyph_id(c).unwrap_or(GlyphId(0));
        if let Some(left) = previous {
            pen += f32::from(face.kerning(left, glyph)) * scale;
        }
        glyphs.push(PlacedGlyph { glyph, x: pen });
        pen += f32::from(face.advance(glyph).unwrap_or(0)) * scale;
        previous = Some(glyph);
    }
    let gap = f32::from(face.line_gap());
    ShapedRun {
        glyphs,
        width: pen,
        ascent: f32::from(face.ascender()) * scale,
        line_height: (f32::from(face.ascender()) - f32::from(face.descender()) + gap) * scale,
    }
}
