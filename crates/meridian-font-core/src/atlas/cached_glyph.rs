//! A glyph that lives on the atlas page.

use super::atlas_rect::AtlasRect;

/// A rasterized glyph on the atlas page, with the placement to draw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedGlyph {
    /// Where its bitmap sits on the page.
    pub rect: AtlasRect,
    /// Pixels from the pen position to the bitmap's left edge.
    pub left: i32,
    /// Pixels from the baseline up to the bitmap's top edge.
    pub top: i32,
}
