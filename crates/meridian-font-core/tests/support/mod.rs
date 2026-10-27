//! A tiny TrueType font assembled in memory, so no real font is committed.
//!
//! Glyphs: 0 empty (`.notdef`), 1 square, 2 quadratic curve, 3 composite of
//! the square moved right by 100, 4 a loop of off-curve points, 5 a
//! composite that includes itself.
//! Characters: `A`..`D` map to glyphs 1..4, `F` to 2, `G` to 1; kerning
//! pair (1, 2) is -50.

/// How the synthetic font stores optional parts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Build {
    pub(crate) long_loca: bool,
    pub(crate) format12: bool,
}

pub(crate) const DEFAULT: Build = Build {
    long_loca: true,
    format12: false,
};

fn be16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn be16s(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn be32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn pad4(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

/// A simple glyph whose points all use 16-bit deltas: `(x, y, on_curve)`.
fn simple_glyph(points: &[(i16, i16, bool)]) -> Vec<u8> {
    let mut out = Vec::new();
    be16s(&mut out, 1);
    for _ in 0..4 {
        be16s(&mut out, 0);
    }
    be16(&mut out, u16::try_from(points.len() - 1).unwrap_or(0));
    be16(&mut out, 0);
    for &(_, _, on) in points {
        out.push(u8::from(on));
    }
    let (mut px, mut py) = (0, 0);
    for &(x, _, _) in points {
        be16s(&mut out, x - px);
        px = x;
    }
    for &(_, y, _) in points {
        be16s(&mut out, y - py);
        py = y;
    }
    pad4(&mut out);
    out
}

fn composite_glyph(component: u16, dx: i16, dy: i16) -> Vec<u8> {
    let mut out = Vec::new();
    be16s(&mut out, -1);
    for _ in 0..4 {
        be16s(&mut out, 0);
    }
    be16(&mut out, 0x0003);
    be16(&mut out, component);
    be16s(&mut out, dx);
    be16s(&mut out, dy);
    pad4(&mut out);
    out
}

fn glyphs() -> Vec<Vec<u8>> {
    vec![
        Vec::new(),
        simple_glyph(&[
            (0, 0, true),
            (500, 0, true),
            (500, 700, true),
            (0, 700, true),
        ]),
        simple_glyph(&[(0, 0, true), (500, 0, false), (500, 500, true)]),
        composite_glyph(1, 100, 0),
        simple_glyph(&[
            (0, 0, false),
            (400, 0, false),
            (400, 400, false),
            (0, 400, false),
        ]),
        composite_glyph(5, 0, 0),
    ]
}

fn head(long_loca: bool) -> Vec<u8> {
    let mut out = Vec::new();
    be32(&mut out, 0x0001_0000);
    be32(&mut out, 0);
    be32(&mut out, 0);
    be32(&mut out, 0x5F0F_3CF5);
    be16(&mut out, 0);
    be16(&mut out, 1000);
    out.extend_from_slice(&[0; 16]);
    out.extend_from_slice(&[0; 8]);
    be16(&mut out, 0);
    be16(&mut out, 8);
    be16s(&mut out, 2);
    be16s(&mut out, i16::from(long_loca));
    be16s(&mut out, 0);
    out
}

fn maxp(glyph_count: u16) -> Vec<u8> {
    let mut out = Vec::new();
    be32(&mut out, 0x0000_5000);
    be16(&mut out, glyph_count);
    out
}

fn hhea() -> Vec<u8> {
    let mut out = Vec::new();
    be32(&mut out, 0x0001_0000);
    be16s(&mut out, 800);
    be16s(&mut out, -200);
    be16s(&mut out, 90);
    out.extend_from_slice(&[0; 26]);
    be16(&mut out, 3);
    out
}

fn hmtx() -> Vec<u8> {
    let mut out = Vec::new();
    for (advance, bearing) in [(500_u16, 0_i16), (600, 10), (700, 20)] {
        be16(&mut out, advance);
        be16s(&mut out, bearing);
    }
    for _ in 0..3 {
        be16s(&mut out, 0);
    }
    out
}

fn kern() -> Vec<u8> {
    let mut out = Vec::new();
    be16(&mut out, 0);
    be16(&mut out, 1);
    be16(&mut out, 0);
    be16(&mut out, 20);
    be16(&mut out, 0x0001);
    be16(&mut out, 1);
    be16(&mut out, 0);
    be16(&mut out, 0);
    be16(&mut out, 0);
    be16(&mut out, 1);
    be16(&mut out, 2);
    be16s(&mut out, -50);
    out
}

fn cmap(format12: bool) -> Vec<u8> {
    let mut out = Vec::new();
    be16(&mut out, 0);
    be16(&mut out, 1);
    be16(&mut out, 3);
    be16(&mut out, if format12 { 10 } else { 1 });
    be32(&mut out, 12);
    if format12 {
        be16(&mut out, 12);
        be16(&mut out, 0);
        be32(&mut out, 40);
        be32(&mut out, 0);
        be32(&mut out, 2);
        for (first, last, glyph) in [(65_u32, 68_u32, 1_u32), (0x1F600, 0x1F600, 3)] {
            be32(&mut out, first);
            be32(&mut out, last);
            be32(&mut out, glyph);
        }
        return out;
    }
    be16(&mut out, 4);
    be16(&mut out, 44);
    be16(&mut out, 0);
    be16(&mut out, 6);
    out.extend_from_slice(&[0; 6]);
    for end in [68_u16, 71, 0xFFFF] {
        be16(&mut out, end);
    }
    be16(&mut out, 0);
    for start in [65_u16, 70, 0xFFFF] {
        be16(&mut out, start);
    }
    for delta in [-64_i16, 0, 1] {
        be16s(&mut out, delta);
    }
    for range_offset in [0_u16, 4, 0] {
        be16(&mut out, range_offset);
    }
    be16(&mut out, 2);
    be16(&mut out, 1);
    out
}

/// The whole font file.
pub(crate) fn font(build: Build) -> Vec<u8> {
    let glyphs = glyphs();
    let mut glyf = Vec::new();
    let mut loca = Vec::new();
    for glyph in &glyphs {
        let offset = u32::try_from(glyf.len()).unwrap_or(0);
        if build.long_loca {
            be32(&mut loca, offset);
        } else {
            be16(&mut loca, u16::try_from(offset / 2).unwrap_or(0));
        }
        glyf.extend_from_slice(glyph);
    }
    let end = u32::try_from(glyf.len()).unwrap_or(0);
    if build.long_loca {
        be32(&mut loca, end);
    } else {
        be16(&mut loca, u16::try_from(end / 2).unwrap_or(0));
    }
    let tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"cmap", cmap(build.format12)),
        (b"glyf", glyf),
        (b"head", head(build.long_loca)),
        (b"hhea", hhea()),
        (b"hmtx", hmtx()),
        (b"kern", kern()),
        (b"loca", loca),
        (b"maxp", maxp(u16::try_from(glyphs.len()).unwrap_or(0))),
    ];
    assemble(&tables)
}

fn assemble(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    be32(&mut out, 0x0001_0000);
    be16(&mut out, u16::try_from(tables.len()).unwrap_or(0));
    out.extend_from_slice(&[0; 6]);
    let mut offset = 12 + 16 * tables.len();
    for (tag, data) in tables {
        out.extend_from_slice(*tag);
        be32(&mut out, 0);
        be32(&mut out, u32::try_from(offset).unwrap_or(0));
        be32(&mut out, u32::try_from(data.len()).unwrap_or(0));
        offset += data.len().next_multiple_of(4);
    }
    for (_, data) in tables {
        out.extend_from_slice(data);
        pad4(&mut out);
    }
    out
}
