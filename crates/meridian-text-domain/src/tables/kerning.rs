//! The legacy `kern` table: horizontal pair adjustments.

use crate::reader::Reader;
use crate::{FontError, GlyphId};

/// Pair adjustments from the first horizontal format-0 subtable.
#[derive(Debug, Clone, Default)]
pub(crate) struct Kerning {
    /// `(left << 16 | right, value)`, sorted by key.
    pairs: Vec<(u32, i16)>,
}

impl Kerning {
    pub(crate) fn parse(kern: &[u8]) -> Result<Self, FontError> {
        let mut reader = Reader::new(kern, "kern table");
        reader.skip(2)?;
        let tables = reader.u16()?;
        for _ in 0..tables {
            reader.skip(2)?;
            let length = usize::from(reader.u16()?);
            let coverage = reader.u16()?;
            let body = reader.bytes(length.saturating_sub(6))?;
            if coverage & 0x0001 != 0 && coverage >> 8 == 0 {
                return Self::parse_pairs(body);
            }
        }
        Ok(Self::default())
    }

    fn parse_pairs(body: &[u8]) -> Result<Self, FontError> {
        let mut reader = Reader::new(body, "kern pairs");
        let count = usize::from(reader.u16()?);
        reader.skip(6)?;
        if reader.remaining() / 6 < count {
            return Err(FontError::Truncated { what: "kern pairs" });
        }
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let left = u32::from(reader.u16()?);
            let right = u32::from(reader.u16()?);
            pairs.push((left << 16 | right, reader.i16()?));
        }
        pairs.sort_by_key(|&(key, _)| key);
        Ok(Self { pairs })
    }

    pub(crate) fn adjustment(&self, left: GlyphId, right: GlyphId) -> i16 {
        let key = u32::from(left.0) << 16 | u32::from(right.0);
        self.pairs
            .binary_search_by_key(&key, |&(key, _)| key)
            .map_or(0, |index| self.pairs[index].1)
    }
}
