//! cmap format 12: segmented coverage (all of Unicode).

use crate::FontError;
use crate::reader::Reader;

#[derive(Debug, Clone)]
pub(crate) struct Format12 {
    /// `(first, last, first glyph)`, sorted by `first`.
    groups: Vec<(u32, u32, u32)>,
}

impl Format12 {
    pub(crate) fn parse(subtable: &[u8]) -> Result<Self, FontError> {
        let mut reader = Reader::new(subtable, "cmap format 12");
        reader.skip(12)?;
        let count =
            usize::try_from(reader.u32()?).map_err(|_| FontError::Malformed("cmap group count"))?;
        if reader.remaining() / 12 < count {
            return Err(FontError::Truncated {
                what: "cmap groups",
            });
        }
        let mut groups = Vec::with_capacity(count);
        for _ in 0..count {
            groups.push((reader.u32()?, reader.u32()?, reader.u32()?));
        }
        groups.sort_by_key(|&(first, ..)| first);
        Ok(Self { groups })
    }

    pub(crate) fn glyph(&self, code: u32) -> Option<u16> {
        let index = self
            .groups
            .partition_point(|&(first, ..)| first <= code)
            .checked_sub(1)?;
        let (first, last, glyph) = self.groups[index];
        if code > last {
            return None;
        }
        u16::try_from(glyph.checked_add(code - first)?).ok()
    }
}
