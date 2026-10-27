//! Reading the synthetic font.

mod support;

use meridian_font_core::{FontError, FontFace, GlyphId, Point, Segment};
use support::{Build, DEFAULT};

/// Uses a result so the call cannot be optimised away or flagged unused.
fn touch<T>(value: T) {
    std::hint::black_box(value);
}

fn face() -> FontFace {
    FontFace::parse(&support::font(DEFAULT)).expect("test font")
}

#[test]
fn global_metrics_come_from_head_and_hhea() {
    let face = face();
    assert_eq!(face.units_per_em(), 1000);
    assert_eq!(face.ascender(), 800);
    assert_eq!(face.descender(), -200);
    assert_eq!(face.line_gap(), 90);
    assert_eq!(face.glyph_count(), 6);
}

#[test]
fn advances_repeat_the_last_metric_for_trailing_glyphs() {
    let face = face();
    assert_eq!(face.advance(GlyphId(1)), Some(600));
    assert_eq!(face.advance(GlyphId(2)), Some(700));
    assert_eq!(face.advance(GlyphId(4)), Some(700));
    assert_eq!(face.advance(GlyphId(6)), None);
}

#[test]
fn format4_maps_deltas_and_glyph_arrays() {
    let face = face();
    assert_eq!(face.glyph_id('A'), Some(GlyphId(1)));
    assert_eq!(face.glyph_id('D'), Some(GlyphId(4)));
    assert_eq!(face.glyph_id('F'), Some(GlyphId(2)));
    assert_eq!(face.glyph_id('G'), Some(GlyphId(1)));
    assert_eq!(face.glyph_id('E'), None);
    assert_eq!(face.glyph_id('z'), None);
}

#[test]
fn format12_covers_beyond_the_basic_plane() {
    let face = FontFace::parse(&support::font(Build {
        format12: true,
        ..DEFAULT
    }))
    .expect("font");
    assert_eq!(face.glyph_id('B'), Some(GlyphId(2)));
    assert_eq!(face.glyph_id('\u{1F600}'), Some(GlyphId(3)));
    assert_eq!(face.glyph_id('\u{1F601}'), None);
}

#[test]
fn short_loca_gives_the_same_glyphs() {
    let long = face().outline(GlyphId(2)).expect("outline");
    let short = FontFace::parse(&support::font(Build {
        long_loca: false,
        ..DEFAULT
    }))
    .expect("font")
    .outline(GlyphId(2))
    .expect("outline");
    assert_eq!(long, short);
}

#[test]
fn kerning_looks_up_pairs() {
    let face = face();
    assert_eq!(face.kerning(GlyphId(1), GlyphId(2)), -50);
    assert_eq!(face.kerning(GlyphId(2), GlyphId(1)), 0);
}

#[test]
fn a_square_is_one_contour_of_lines() {
    let outline = face().outline(GlyphId(1)).expect("outline");
    assert_eq!(outline.contours.len(), 1);
    let contour = &outline.contours[0];
    assert_eq!(contour.start, Point::new(0.0, 0.0));
    assert_eq!(contour.segments.len(), 3);
    assert!(
        contour
            .segments
            .iter()
            .all(|s| matches!(s, Segment::Line { .. }))
    );
    assert_eq!(
        outline.bounds(),
        Some((Point::new(0.0, 0.0), Point::new(500.0, 700.0)))
    );
}

#[test]
fn an_off_curve_point_becomes_a_quadratic_segment() {
    let outline = face().outline(GlyphId(2)).expect("outline");
    assert_eq!(
        outline.contours[0].segments,
        vec![Segment::Quad {
            control: Point::new(500.0, 0.0),
            to: Point::new(500.0, 500.0)
        }]
    );
}

#[test]
fn a_loop_of_off_curve_points_starts_halfway_between_ends() {
    let outline = face().outline(GlyphId(4)).expect("outline");
    let contour = &outline.contours[0];
    assert_eq!(contour.start, Point::new(0.0, 200.0));
    assert_eq!(contour.segments.len(), 4);
    assert_eq!(contour.segments[0].end(), Point::new(200.0, 0.0));
    assert_eq!(contour.segments[3].end(), contour.start);
}

#[test]
fn a_composite_places_its_component() {
    let outline = face().outline(GlyphId(3)).expect("outline");
    assert_eq!(
        outline.bounds(),
        Some((Point::new(100.0, 0.0), Point::new(600.0, 700.0)))
    );
}

#[test]
fn an_empty_glyph_draws_nothing() {
    assert!(face().outline(GlyphId(0)).expect("outline").is_empty());
}

#[test]
fn a_self_including_composite_is_an_error_not_a_hang() {
    assert!(matches!(
        face().outline(GlyphId(5)),
        Err(FontError::Malformed(_))
    ));
}

#[test]
fn an_unknown_glyph_is_out_of_range() {
    assert_eq!(
        face().outline(GlyphId(99)),
        Err(FontError::GlyphOutOfRange(99))
    );
}

#[test]
fn non_font_bytes_are_rejected() {
    assert!(FontFace::parse(b"").is_err());
    assert!(matches!(
        FontFace::parse(b"OTTO\0\0\0\0\0\0\0\0"),
        Err(FontError::Unsupported(_))
    ));
}

#[test]
fn a_missing_table_is_named() {
    let mut bytes = support::font(DEFAULT);
    let kern = bytes
        .windows(4)
        .position(|w| w == b"kern")
        .expect("kern entry");
    bytes[kern..kern + 4].copy_from_slice(b"cmap");
    // The directory now lists `cmap` twice and no `kern`; the first `cmap` wins.
    assert!(FontFace::parse(&bytes).is_ok());
    let hmtx = bytes
        .windows(4)
        .position(|w| w == b"hmtx")
        .expect("hmtx entry");
    bytes[hmtx..hmtx + 4].copy_from_slice(b"zzzz");
    assert_eq!(
        FontFace::parse(&bytes).err(),
        Some(FontError::MissingTable("hmtx"))
    );
}

#[test]
fn every_truncation_and_corruption_is_an_error_or_a_face_never_a_panic() {
    let bytes = support::font(DEFAULT);
    for len in 0..bytes.len() {
        touch(FontFace::parse(&bytes[..len]));
    }
    for at in 0..bytes.len() {
        for value in [0x00, 0x7F, 0xFF] {
            let mut copy = bytes.clone();
            copy[at] = value;
            if let Ok(face) = FontFace::parse(&copy) {
                for glyph in 0..face.glyph_count() {
                    touch(face.outline(GlyphId(glyph)));
                    touch(face.advance(GlyphId(glyph)));
                }
                touch(face.glyph_id('A'));
                touch(face.glyph_id('\u{1F600}'));
            }
        }
    }
}
