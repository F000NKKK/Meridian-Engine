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
        hhea.skip(24)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_standard_hhea_table_exposes_its_horizontal_metric_count() {
        let mut head = [0u8; 54];
        head[18..20].copy_from_slice(&1000u16.to_be_bytes());
        head[50..52].copy_from_slice(&1i16.to_be_bytes());
        let mut maxp = [0u8; 6];
        maxp[4..6].copy_from_slice(&12u16.to_be_bytes());
        let mut hhea = [0u8; 36];
        hhea[4..6].copy_from_slice(&800i16.to_be_bytes());
        hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes());
        hhea[34..36].copy_from_slice(&12u16.to_be_bytes());
        let header = Header::parse(&head, &maxp, &hhea).expect("a standard 36-byte hhea table");
        assert_eq!(header.ascender, 800);
        assert_eq!(header.descender, -200);
        assert_eq!(header.metric_count, 12);
    }
}
