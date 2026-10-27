//! cmap format 4: segment mapping to delta values (the Basic Multilingual Plane).

use crate::FontError;
use crate::reader::Reader;

#[derive(Debug, Clone)]
pub(crate) struct Format4 {
    end_codes: Vec<u16>,
    start_codes: Vec<u16>,
    deltas: Vec<i16>,
    range_offsets: Vec<u16>,
    glyph_ids: Vec<u16>,
}

impl Format4 {
    pub(crate) fn parse(subtable: &[u8]) -> Result<Self, FontError> {
        let mut reader = Reader::new(subtable, "cmap format 4");
        reader.skip(2)?;
        let length = usize::from(reader.u16()?).min(subtable.len());
        reader.skip(2)?;
        let segments = usize::from(reader.u16()? / 2);
        reader.skip(6)?;
        let end_codes = read_u16s(&mut reader, segments)?;
        reader.skip(2)?;
        let start_codes = read_u16s(&mut reader, segments)?;
        let deltas = (0..segments)
            .map(|_| reader.i16())
            .collect::<Result<Vec<_>, _>>()?;
        let range_offsets = read_u16s(&mut reader, segments)?;
        let consumed = 16 + 8 * segments;
        let glyph_count = length.saturating_sub(consumed) / 2;
        let glyph_count = glyph_count.min(reader.remaining() / 2);
        let glyph_ids = read_u16s(&mut reader, glyph_count)?;
        Ok(Self {
            end_codes,
            start_codes,
            deltas,
            range_offsets,
            glyph_ids,
        })
    }

    pub(crate) fn glyph(&self, code: u32) -> Option<u16> {
        let code = u16::try_from(code).ok()?;
        let segment = self.end_codes.iter().position(|&end| end >= code)?;
        let start = self.start_codes[segment];
        if start > code {
            return None;
        }
        let delta = self.deltas[segment];
        let range_offset = usize::from(self.range_offsets[segment]);
        if range_offset == 0 {
            return Some(code.wrapping_add_signed(delta));
        }
        let index = (range_offset / 2 + usize::from(code - start) + segment)
            .checked_sub(self.end_codes.len())?;
        let glyph = *self.glyph_ids.get(index)?;
        (glyph != 0).then(|| glyph.wrapping_add_signed(delta))
    }
}

fn read_u16s(reader: &mut Reader<'_>, count: usize) -> Result<Vec<u16>, FontError> {
    if reader.remaining() / 2 < count {
        return Err(FontError::Truncated {
            what: "cmap format 4",
        });
    }
    (0..count).map(|_| reader.u16()).collect()
}
