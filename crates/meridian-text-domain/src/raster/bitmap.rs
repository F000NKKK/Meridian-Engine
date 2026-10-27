//! A rasterized glyph.

/// A rasterized glyph: 8-bit coverage, rows top to bottom.
///
/// `left` and `top` place it relative to the glyph origin on the baseline:
/// the first column starts `left` pixels right of the origin and the first
/// row's top edge is `top` pixels above the baseline.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bitmap {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// Pixels from the origin to the left edge; negative is left of it.
    pub left: i32,
    /// Pixels from the baseline up to the top edge.
    pub top: i32,
    /// `width * height` coverage values, 0 empty to 255 full.
    pub coverage: Vec<u8>,
}
