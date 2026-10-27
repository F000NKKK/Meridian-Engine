//! What extraction needs to draw text.

use meridian_font_core::{FontFace, GlyphCache};
use meridian_graphics_core::TextureHandle;

/// The font, the glyph cache and the texture the cache's page is uploaded
/// to. The caller uploads `cache.atlas().pixels()` whenever
/// `cache.version()` changes.
#[derive(Debug)]
pub struct TextPaint<'a> {
    /// The face every `TextBlock` is set in.
    pub face: &'a FontFace,
    /// Rasterized glyphs and their page.
    pub cache: &'a mut GlyphCache,
    /// The texture holding the page (single channel coverage).
    pub atlas: TextureHandle,
}
