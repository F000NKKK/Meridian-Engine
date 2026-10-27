//! A glyph's index in its face.

/// A glyph's index in its face. `0` is the face's "missing glyph".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphId(pub u16);
