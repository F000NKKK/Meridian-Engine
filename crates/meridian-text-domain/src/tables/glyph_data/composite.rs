//! Composite glyphs: other glyphs placed, scaled and combined.

use crate::reader::Reader;
use crate::{FontError, Outline};

const ARGS_ARE_WORDS: u16 = 0x0001;
const ARGS_ARE_XY: u16 = 0x0002;
const HAVE_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const HAVE_XY_SCALE: u16 = 0x0040;
const HAVE_TWO_BY_TWO: u16 = 0x0080;

/// More components than any real glyph needs; guards against endless chains.
const MAX_COMPONENTS: usize = 64;

/// Reads a composite glyph's components and combines their outlines;
/// `component` resolves a glyph id to its outline.
pub(super) fn read(
    reader: &mut Reader<'_>,
    component: impl Fn(u16) -> Result<Outline, FontError>,
) -> Result<Outline, FontError> {
    let mut outline = Outline::default();
    for _ in 0..MAX_COMPONENTS {
        let flags = reader.u16()?;
        let glyph = reader.u16()?;
        if flags & ARGS_ARE_XY == 0 {
            return Err(FontError::Unsupported("composite glyph point matching"));
        }
        let (dx, dy) = read_offset(reader, flags)?;
        let matrix = read_matrix(reader, flags)?;
        let mut part = component(glyph)?;
        part.transform(matrix);
        part.translate(dx, dy);
        outline.contours.extend(part.contours);
        if flags & MORE_COMPONENTS == 0 {
            return Ok(outline);
        }
    }
    Err(FontError::Malformed("too many glyph components"))
}

fn read_offset(reader: &mut Reader<'_>, flags: u16) -> Result<(f32, f32), FontError> {
    if flags & ARGS_ARE_WORDS != 0 {
        Ok((f32::from(reader.i16()?), f32::from(reader.i16()?)))
    } else {
        Ok((f32::from(reader.i8()?), f32::from(reader.i8()?)))
    }
}

/// `[xx, xy, yx, yy]`, identity unless the flags give a scale.
fn read_matrix(reader: &mut Reader<'_>, flags: u16) -> Result<[f32; 4], FontError> {
    if flags & HAVE_SCALE != 0 {
        let scale = f2dot14(reader)?;
        Ok([scale, 0.0, 0.0, scale])
    } else if flags & HAVE_XY_SCALE != 0 {
        let x = f2dot14(reader)?;
        Ok([x, 0.0, 0.0, f2dot14(reader)?])
    } else if flags & HAVE_TWO_BY_TWO != 0 {
        Ok([
            f2dot14(reader)?,
            f2dot14(reader)?,
            f2dot14(reader)?,
            f2dot14(reader)?,
        ])
    } else {
        Ok([1.0, 0.0, 0.0, 1.0])
    }
}

fn f2dot14(reader: &mut Reader<'_>) -> Result<f32, FontError> {
    Ok(f32::from(reader.i16()?) / 16384.0)
}
