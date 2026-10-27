//! The `cmap` table: character to glyph.

use super::format4::Format4;
use super::format12::Format12;
use crate::reader::Reader;
use crate::{FontError, GlyphId};

/// A character map: the best Unicode subtable the font offers.
#[derive(Debug, Clone)]
pub(crate) enum Cmap {
    Format4(Format4),
    Format12(Format12),
}

impl Cmap {
    pub(crate) fn parse(cmap: &[u8]) -> Result<Self, FontError> {
        let mut reader = Reader::new(cmap, "cmap table");
        reader.skip(2)?;
        let count = reader.u16()?;
        let mut best: Option<Self> = None;
        for _ in 0..count {
            let platform = reader.u16()?;
            let encoding = reader.u16()?;
            let offset = usize::try_from(reader.u32()?)
                .map_err(|_| FontError::Malformed("cmap subtable offset"))?;
            if !is_unicode(platform, encoding) {
                continue;
            }
            let subtable = cmap.get(offset..).ok_or(FontError::Truncated {
                what: "cmap subtable",
            })?;
            let parsed = match Reader::new(subtable, "cmap subtable").u16()? {
                12 => Self::Format12(Format12::parse(subtable)?),
                4 => Self::Format4(Format4::parse(subtable)?),
                _ => continue,
            };
            if best.is_none() || matches!(parsed, Self::Format12(_)) {
                best = Some(parsed);
            }
        }
        best.ok_or(FontError::Unsupported("no Unicode cmap subtable"))
    }

    /// The glyph for `c`, `None` when the font does not cover it.
    pub(crate) fn glyph(&self, c: char) -> Option<GlyphId> {
        let id = match self {
            Self::Format4(table) => table.glyph(u32::from(c)),
            Self::Format12(table) => table.glyph(u32::from(c)),
        };
        id.filter(|&id| id != 0).map(GlyphId)
    }
}

fn is_unicode(platform: u16, encoding: u16) -> bool {
    matches!((platform, encoding), (0, _) | (3, 1 | 10))
}
