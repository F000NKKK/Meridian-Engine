//! The `loca` table: where each glyph sits in `glyf`.

use crate::FontError;
use crate::reader::Reader;

/// `glyph_count + 1` byte offsets into `glyf`, non-decreasing and inside it.
pub(crate) fn parse(
    loca: &[u8],
    glyph_count: u16,
    long: bool,
    glyf_len: usize,
) -> Result<Vec<usize>, FontError> {
    let mut reader = Reader::new(loca, "loca table");
    let mut offsets = Vec::with_capacity(usize::from(glyph_count) + 1);
    for _ in 0..=glyph_count {
        let offset = if long {
            usize::try_from(reader.u32()?).map_err(|_| FontError::Malformed("loca offset"))?
        } else {
            usize::from(reader.u16()?) * 2
        };
        offsets.push(offset);
    }
    let ordered = offsets.windows(2).all(|pair| pair[0] <= pair[1]);
    if !ordered || offsets.last().is_some_and(|&end| end > glyf_len) {
        return Err(FontError::Malformed("loca offsets"));
    }
    Ok(offsets)
}
