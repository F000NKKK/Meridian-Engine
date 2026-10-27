//! Text blocks become glyph sprites.
#![expect(
    clippy::float_cmp,
    reason = "glyph origins are snapped to whole pixels, so positions are exact"
)]

use meridian_memory::Handle;
use meridian_rendering_domain::{Scene2d, TextureHandle};
use meridian_text_domain::synthetic::{DEFAULT, font};
use meridian_text_domain::{FontFace, GlyphCache};
use meridian_ui_domain::layout::value::Visibility;
use meridian_ui_domain::properties::element::VISIBILITY;
use meridian_ui_domain::properties::text_block::{FONT_SIZE, FOREGROUND, TEXT};
use meridian_ui_domain::{Color, ElementKind, NodeId, Size, TextMeasurer, UiTree, compute_layout};
use meridian_ui_scene2d::{FaceMeasurer, RenderError, TextPaint, UiPaint, extract_scene_with_text};

fn texture(index: u32) -> TextureHandle {
    TextureHandle::new(Handle {
        index,
        generation: 0,
    })
}

fn face() -> FontFace {
    FontFace::parse(&font(DEFAULT)).expect("test font")
}

fn text_block(tree: &mut UiTree, text: &str, size: f32) -> NodeId {
    let node = tree.create(ElementKind::TextBlock);
    tree.set(node, TEXT, text.to_string()).expect("test setup");
    tree.set(node, FONT_SIZE, size).expect("test setup");
    tree.set(node, FOREGROUND, Color::new(0.0, 1.0, 0.0, 1.0))
        .expect("test setup");
    node
}

fn extract(
    tree: &mut UiTree,
    root: NodeId,
    face: &FontFace,
    cache: &mut GlyphCache,
) -> Result<Scene2d, RenderError> {
    let surface = Size::new(300.0, 200.0);
    compute_layout(tree, root, surface, &FaceMeasurer::new(face)).expect("test setup");
    let mut text = TextPaint {
        face,
        cache,
        atlas: texture(9),
    };
    extract_scene_with_text(tree, root, surface, UiPaint { fill: texture(7) }, &mut text)
}

#[test]
fn the_measurer_reports_the_run_width_and_line_height() {
    let face = face();
    let size = FaceMeasurer::new(&face).measure("AB", 100.0);
    assert!((size.width - 125.0).abs() < 1e-3);
    assert!((size.height - 109.0).abs() < 1e-3);
}

#[test]
fn each_glyph_is_a_sprite_on_the_atlas_texture() {
    let (face, mut tree) = (face(), UiTree::new());
    let mut cache = GlyphCache::new(256, 256);
    let root = text_block(&mut tree, "AB", 100.0);
    let scene = extract(&mut tree, root, &face, &mut cache).expect("scene");
    assert_eq!(scene.sprites.len(), 2);
    let tint = Color::new(0.0, 1.0, 0.0, 1.0).to_array();
    for sprite in &scene.sprites {
        assert_eq!(sprite.texture, texture(9));
        assert_eq!(sprite.tint, tint);
        assert!(sprite.source_rect.is_some());
    }
    let (a, b) = (&scene.sprites[0], &scene.sprites[1]);
    assert_eq!(a.size, [50.0, 70.0]);
    assert_eq!(b.size, [50.0, 50.0]);
    assert_eq!((a.layer, b.layer), (0, 1));
}

#[test]
fn glyphs_sit_on_the_baseline_at_their_pen_positions() {
    let (face, mut tree) = (face(), UiTree::new());
    let mut cache = GlyphCache::new(256, 256);
    let root = text_block(&mut tree, "AB", 100.0);
    let scene = extract(&mut tree, root, &face, &mut cache).expect("scene");
    let center = |index: usize| {
        let point = scene.sprites[index]
            .frame
            .transform_point(meridian_gac::Vec3::new(0.0, 0.0, 0.0));
        (point.x, point.y)
    };
    // Ascent is 80 px, A is 70 px tall: top at 10, centre at 45. B starts 55 px along.
    assert_eq!(center(0), (25.0, 45.0));
    assert_eq!(center(1), (80.0, 55.0));
}

#[test]
fn the_source_rect_points_into_the_atlas_page() {
    let (face, mut tree) = (face(), UiTree::new());
    let mut cache = GlyphCache::new(256, 128);
    let root = text_block(&mut tree, "A", 100.0);
    let scene = extract(&mut tree, root, &face, &mut cache).expect("scene");
    assert_eq!(
        scene.sprites[0].source_rect,
        Some([0.0, 0.0, 50.0 / 256.0, 70.0 / 128.0])
    );
}

#[test]
fn hidden_text_draws_nothing() {
    let (face, mut tree) = (face(), UiTree::new());
    let mut cache = GlyphCache::new(256, 256);
    let root = text_block(&mut tree, "AB", 100.0);
    tree.set(root, VISIBILITY, Visibility::Hidden)
        .expect("test setup");
    assert!(
        extract(&mut tree, root, &face, &mut cache)
            .expect("scene")
            .sprites
            .is_empty()
    );
}

#[test]
fn spaces_and_unknown_characters_add_no_sprites_beyond_the_missing_glyph() {
    let (face, mut tree) = (face(), UiTree::new());
    let mut cache = GlyphCache::new(256, 256);
    let root = text_block(&mut tree, "  ", 100.0);
    // The synthetic font has no space glyph: each maps to the empty missing glyph.
    assert!(
        extract(&mut tree, root, &face, &mut cache)
            .expect("scene")
            .sprites
            .is_empty()
    );
}

#[test]
fn a_page_too_small_for_the_text_is_an_error() {
    let (face, mut tree) = (face(), UiTree::new());
    let mut cache = GlyphCache::new(60, 80);
    let root = text_block(&mut tree, "AB", 100.0);
    assert!(matches!(
        extract(&mut tree, root, &face, &mut cache),
        Err(RenderError::Font(_))
    ));
}

#[test]
fn a_stale_root_is_a_tree_error() {
    let (face, mut tree) = (face(), UiTree::new());
    let mut cache = GlyphCache::new(64, 64);
    let root = text_block(&mut tree, "A", 10.0);
    tree.remove(root).expect("test setup");
    let mut text = TextPaint {
        face: &face,
        cache: &mut cache,
        atlas: texture(1),
    };
    let result = extract_scene_with_text(
        &tree,
        root,
        Size::new(10.0, 10.0),
        UiPaint { fill: texture(2) },
        &mut text,
    );
    assert!(matches!(result, Err(RenderError::Tree(_))));
}
