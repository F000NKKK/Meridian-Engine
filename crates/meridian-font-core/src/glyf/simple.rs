//! Simple glyphs: contours of flagged points.

use super::contour_builder::{RawPoint, build};
use crate::reader::Reader;
use crate::{FontError, Outline, Point};

const ON_CURVE: u8 = 0x01;
const X_SHORT: u8 = 0x02;
const Y_SHORT: u8 = 0x04;
const REPEAT: u8 = 0x08;
const X_SAME_OR_POSITIVE: u8 = 0x10;
const Y_SAME_OR_POSITIVE: u8 = 0x20;

/// Reads a simple glyph's body; `reader` sits just after the glyph header.
pub(super) fn read(reader: &mut Reader<'_>, contour_count: usize) -> Result<Outline, FontError> {
    let mut ends = Vec::with_capacity(contour_count);
    for _ in 0..contour_count {
        ends.push(usize::from(reader.u16()?));
    }
    if ends.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(FontError::Malformed("contour end points"));
    }
    let point_count = ends.last().map_or(0, |&end| end + 1);
    let instructions = usize::from(reader.u16()?);
    reader.skip(instructions)?;

    let flags = read_flags(reader, point_count)?;
    let xs = read_axis(reader, &flags, X_SHORT, X_SAME_OR_POSITIVE)?;
    let ys = read_axis(reader, &flags, Y_SHORT, Y_SAME_OR_POSITIVE)?;
    let points: Vec<RawPoint> = flags
        .iter()
        .zip(xs.iter().zip(&ys))
        .map(|(&flag, (&x, &y))| RawPoint {
            at: Point::new(x, y),
            on_curve: flag & ON_CURVE != 0,
        })
        .collect();

    let mut contours = Vec::with_capacity(contour_count);
    let mut begin = 0;
    for end in ends {
        contours.extend(build(&points[begin..=end]));
        begin = end + 1;
    }
    Ok(Outline { contours })
}

fn read_flags(reader: &mut Reader<'_>, count: usize) -> Result<Vec<u8>, FontError> {
    let mut flags = Vec::with_capacity(count);
    while flags.len() < count {
        let flag = reader.u8()?;
        flags.push(flag);
        if flag & REPEAT != 0 {
            let repeats = usize::from(reader.u8()?).min(count - flags.len());
            flags.resize(flags.len() + repeats, flag);
        }
    }
    Ok(flags)
}

/// Absolute coordinates along one axis, accumulated from the deltas.
fn read_axis(
    reader: &mut Reader<'_>,
    flags: &[u8],
    short: u8,
    same_or_positive: u8,
) -> Result<Vec<f32>, FontError> {
    let mut value = 0.0_f32;
    let mut values = Vec::with_capacity(flags.len());
    for &flag in flags {
        let delta = if flag & short != 0 {
            let magnitude = f32::from(reader.u8()?);
            if flag & same_or_positive != 0 {
                magnitude
            } else {
                -magnitude
            }
        } else if flag & same_or_positive != 0 {
            0.0
        } else {
            f32::from(reader.i16()?)
        };
        value += delta;
        values.push(value);
    }
    Ok(values)
}
