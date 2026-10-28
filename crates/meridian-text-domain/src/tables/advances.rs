//! The `hmtx` table: horizontal advance of every glyph.

use crate::FontError;
use crate::reader::Reader;

/// Advance widths in font units, one per glyph.
pub(crate) fn parse(
    hmtx: &[u8],
    glyph_count: u16,
    metric_count: u16,
) -> Result<Vec<u16>, FontError> {
    if metric_count == 0 || metric_count > glyph_count {
        return Err(FontError::Malformed("number of horizontal metrics"));
    }
    let mut reader = Reader::new(hmtx, "hmtx table");
    let mut advances = Vec::with_capacity(usize::from(glyph_count));
    for _ in 0..metric_count {
        advances.push(reader.u16()?);
        reader.skip(2)?;
    }
    let last = advances.last().copied().unwrap_or(0);
    advances.resize(usize::from(glyph_count), last);
    Ok(advances)
}
