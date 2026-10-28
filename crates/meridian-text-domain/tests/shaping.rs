//! Shaping text with the synthetic font.

use meridian_text_domain::synthetic::{DEFAULT, font};
use meridian_text_domain::{FontFace, GlyphId, shape};

fn face() -> FontFace {
    FontFace::parse(&font(DEFAULT)).expect("test font")
}

#[test]
fn glyphs_advance_by_their_widths_scaled_to_the_size() {
    // 1000 units per em, so 100 px means 0.1 px per unit.
    let run = shape(&face(), "AB", 100.0);
    let glyphs: Vec<_> = run.glyphs.iter().map(|g| g.glyph).collect();
    assert_eq!(glyphs, vec![GlyphId(1), GlyphId(2)]);
    assert!((run.glyphs[0].x - 0.0).abs() < 1e-4);
    // A is 600 units wide, kerned by -50 against B.
    assert!((run.glyphs[1].x - 55.0).abs() < 1e-3);
    assert!((run.width - (55.0 + 70.0)).abs() < 1e-3);
}

#[test]
fn unknown_characters_use_the_missing_glyph() {
    let run = shape(&face(), "z", 10.0);
    assert_eq!(run.glyphs[0].glyph, GlyphId(0));
    assert!((run.width - 5.0).abs() < 1e-4);
}

#[test]
fn control_characters_draw_nothing() {
    let run = shape(&face(), "A\nB\t", 10.0);
    assert_eq!(run.glyphs.len(), 2);
}

#[test]
fn empty_text_has_no_width_but_keeps_line_metrics() {
    let run = shape(&face(), "", 100.0);
    assert_eq!(run.glyphs, Vec::new());
    assert!(run.width.abs() < 1e-6);
    assert!((run.ascent - 80.0).abs() < 1e-3);
    assert!((run.line_height - 109.0).abs() < 1e-3);
}
