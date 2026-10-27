//! The `head`, `maxp` and `hhea` tables: global metrics.

use crate::FontError;
use crate::reader::Reader;

/// Global font facts read from `head`, `maxp` and `hhea`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Header {
    pub(crate) units_per_em: u16,
    pub(crate) long_loca: bool,
    pub(crate) glyph_count: u16,
    pub(crate) ascender: i16,
    pub(crate) descender: i16,
    pub(crate) line_gap: i16,
    pub(crate) metric_count: u16,
}

impl Header {
    pub(crate) fn parse(head: &[u8], maxp: &[u8], hhea: &[u8]) -> Result<Self, FontError> {
        let mut head = Reader::new(head, "head table");
        head.skip(18)?;
        let units_per_em = head.u16()?;
        if units_per_em == 0 {
            return Err(FontError::Malformed("units per em is zero"));
        }
        head.skip(30)?;
        let long_loca = match head.i16()? {
            0 => false,
            1 => true,
            _ => return Err(FontError::Malformed("index to loc format")),
        };

        let mut maxp = Reader::new(maxp, "maxp table");
        maxp.skip(4)?;
        let glyph_count = maxp.u16()?;

        let mut hhea = Reader::new(hhea, "hhea table");
        hhea.skip(4)?;
        let ascender = hhea.i16()?;
        let descender = hhea.i16()?;
        let line_gap = hhea.i16()?;
        hhea.skip(26)?;
        let metric_count = hhea.u16()?;

        Ok(Self {
            units_per_em,
            long_loca,
            glyph_count,
            ascender,
            descender,
            line_gap,
            metric_count,
        })
    }
}
