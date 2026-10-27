//! Packing and caching glyph bitmaps.

#![expect(
    clippy::float_cmp,
    reason = "normalized rects are exact binary fractions"
)]

use meridian_font_core::synthetic::{DEFAULT, font};
use meridian_font_core::{Bitmap, FontError, FontFace, GlyphAtlas, GlyphCache, GlyphId};

fn block(width: u32, height: u32, value: u8) -> Bitmap {
    Bitmap {
        width,
        height,
        left: 0,
        top: 0,
        coverage: vec![value; (width * height) as usize],
    }
}

fn face() -> FontFace {
    FontFace::parse(&font(DEFAULT)).expect("test font")
}

#[test]
fn bitmaps_are_copied_to_their_slot() {
    let mut atlas = GlyphAtlas::new(16, 16);
    let rect = atlas.insert(&block(3, 2, 9)).expect("fits");
    assert_eq!((rect.x, rect.y, rect.width, rect.height), (0, 0, 3, 2));
    let page = atlas.pixels();
    assert_eq!(&page[..3], &[9, 9, 9]);
    assert_eq!(&page[16..19], &[9, 9, 9]);
    assert_eq!(page[3], 0, "padding stays empty");
    assert_eq!(page[32], 0);
}

#[test]
fn bitmaps_share_a_shelf_then_open_a_new_one() {
    let mut atlas = GlyphAtlas::new(10, 20);
    let first = atlas.insert(&block(4, 4, 1)).expect("fits");
    let second = atlas.insert(&block(4, 4, 2)).expect("fits");
    assert_eq!((first.x, second.x), (0, 5));
    assert_eq!(first.y, second.y);
    let third = atlas.insert(&block(4, 4, 3)).expect("fits");
    assert_eq!((third.x, third.y), (0, 5));
}

#[test]
fn a_bitmap_that_cannot_fit_is_refused() {
    let mut atlas = GlyphAtlas::new(8, 8);
    assert!(
        atlas.insert(&block(8, 2, 1)).is_none(),
        "no room for padding"
    );
    assert!(atlas.insert(&block(4, 8, 1)).is_none());
    assert!(atlas.insert(&block(4, 4, 1)).is_some());
}

#[test]
fn an_empty_bitmap_takes_no_room() {
    let mut atlas = GlyphAtlas::new(2, 2);
    assert!(atlas.insert(&Bitmap::default()).is_some());
    assert!(atlas.insert(&block(1, 1, 7)).is_some());
}

#[test]
fn clearing_blanks_the_page_and_frees_the_room() {
    let mut atlas = GlyphAtlas::new(8, 8);
    atlas.insert(&block(5, 5, 200)).expect("fits");
    atlas.clear();
    assert!(atlas.pixels().iter().all(|&p| p == 0));
    assert!(atlas.insert(&block(5, 5, 1)).is_some());
}

#[test]
fn a_rect_normalizes_to_a_source_rect() {
    let mut atlas = GlyphAtlas::new(64, 32);
    atlas.insert(&block(8, 4, 1)).expect("fits");
    let rect = atlas.insert(&block(16, 8, 1)).expect("fits");
    assert_eq!(
        rect.normalized(64, 32),
        [0.0, 5.0 / 32.0, 16.0 / 64.0, 8.0 / 32.0]
    );
}

#[test]
fn the_cache_rasterizes_once_per_size() {
    let face = face();
    let mut cache = GlyphCache::new(128, 128);
    let first = cache.glyph(&face, GlyphId(1), 100.0).expect("glyph");
    assert_eq!((first.rect.width, first.rect.height), (50, 70));
    assert_eq!((first.left, first.top), (0, 70));
    let version = cache.version();
    assert_eq!(cache.glyph(&face, GlyphId(1), 100.0).expect("glyph"), first);
    assert_eq!(cache.version(), version);
    let smaller = cache.glyph(&face, GlyphId(1), 50.0).expect("glyph");
    assert_eq!((smaller.rect.width, smaller.rect.height), (25, 35));
    assert!(cache.version() > version);
    assert_eq!(cache.atlas().pixels()[first.rect.y as usize * 128], 255);
}

#[test]
fn a_full_page_resets_and_bumps_the_version() {
    let face = face();
    let mut cache = GlyphCache::new(64, 80);
    let first = cache.glyph(&face, GlyphId(1), 100.0).expect("glyph");
    let before = cache.version();
    let second = cache.glyph(&face, GlyphId(2), 100.0).expect("glyph");
    assert!(cache.version() > before + 1, "the reset is a second change");
    assert_eq!((second.rect.x, second.rect.y), (0, 0));
    assert_ne!(first.rect.width, 0);
}

#[test]
fn a_glyph_larger_than_the_page_is_an_error() {
    let face = face();
    let mut cache = GlyphCache::new(16, 16);
    assert_eq!(
        cache.glyph(&face, GlyphId(1), 100.0),
        Err(FontError::BitmapTooLarge)
    );
}

#[test]
fn unknown_glyphs_are_errors() {
    let mut cache = GlyphCache::new(16, 16);
    assert_eq!(
        cache.glyph(&face(), GlyphId(77), 10.0),
        Err(FontError::GlyphOutOfRange(77))
    );
}
