//! The `glyf` table: glyph outlines.

mod composite;
mod contour_builder;
mod simple;

use crate::reader::Reader;
use crate::{FontError, Outline};

/// Composite glyphs may nest this deep; deeper is treated as a cycle.
const MAX_DEPTH: u8 = 8;

/// The `glyf` bytes with the offsets that slice them per glyph.
#[derive(Debug, Clone)]
pub(crate) struct Glyf {
    data: Vec<u8>,
    offsets: Vec<usize>,
}

impl Glyf {
    pub(crate) const fn new(data: Vec<u8>, offsets: Vec<usize>) -> Self {
        Self { data, offsets }
    }

    pub(crate) fn outline(&self, glyph: u16) -> Result<Outline, FontError> {
        self.outline_at_depth(glyph, 0)
    }

    fn outline_at_depth(&self, glyph: u16, depth: u8) -> Result<Outline, FontError> {
        if depth > MAX_DEPTH {
            return Err(FontError::Malformed("composite glyph nesting"));
        }
        let index = usize::from(glyph);
        let (Some(&start), Some(&end)) = (self.offsets.get(index), self.offsets.get(index + 1))
        else {
            return Err(FontError::GlyphOutOfRange(glyph));
        };
        let bytes = self
            .data
            .get(start..end)
            .ok_or(FontError::Malformed("loca offsets"))?;
        if bytes.is_empty() {
            return Ok(Outline::default());
        }
        let mut reader = Reader::new(bytes, "glyph header");
        let contours = reader.i16()?;
        reader.skip(8)?;
        if contours >= 0 {
            simple::read(&mut reader, usize::from(contours.unsigned_abs()))
        } else {
            composite::read(&mut reader, |component| {
                self.outline_at_depth(component, depth + 1)
            })
        }
    }
}
