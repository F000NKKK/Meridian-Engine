//! Scroll viewers clip their content.
#![expect(
    clippy::float_cmp,
    reason = "extraction copies layout floats verbatim, so exact equality is the contract"
)]

use meridian_memory::Handle;
use meridian_rendering_domain::{SpriteBatch, TextureHandle, batch_scene2d_clipped};
use meridian_ui_domain::layout::value::Length;
use meridian_ui_domain::properties::element::{BACKGROUND, HEIGHT, WIDTH};
use meridian_ui_domain::properties::scroll_viewer::VERTICAL_OFFSET;
use meridian_ui_domain::{
    Color, ElementKind, FixedAdvanceText, NodeId, Size, UiTree, compute_layout,
};
use meridian_ui_scene2d::{UiPaint, extract_clipped};

fn paint() -> UiPaint {
    UiPaint {
        fill: TextureHandle::new(Handle {
            index: 3,
            generation: 0,
        }),
    }
}

fn filled(tree: &mut UiTree, parent: NodeId, width: f32, height: f32) -> NodeId {
    let node = tree.create(ElementKind::Border);
    tree.set(node, WIDTH, Length::Px(width))
        .expect("test setup");
    tree.set(node, HEIGHT, Length::Px(height))
        .expect("test setup");
    tree.set(node, BACKGROUND, Color::WHITE)
        .expect("test setup");
    tree.append_child(parent, node).expect("test setup");
    node
}

/// A white 40x40 viewer with a background, over a stack of 4 rows 20 high.
fn viewer_over_rows(tree: &mut UiTree) -> NodeId {
    let viewer = tree.create(ElementKind::ScrollViewer);
    tree.set(viewer, WIDTH, Length::Px(40.0))
        .expect("test setup");
    tree.set(viewer, HEIGHT, Length::Px(40.0))
        .expect("test setup");
    tree.set(viewer, BACKGROUND, Color::BLACK)
        .expect("test setup");
    let stack = tree.create(ElementKind::StackPanel);
    tree.append_child(viewer, stack).expect("test setup");
    for _ in 0..4 {
        filled(tree, stack, 40.0, 20.0);
    }
    viewer
}

#[test]
fn a_viewer_clips_the_sprites_of_its_content_but_not_its_own_background() {
    let mut tree = UiTree::new();
    let viewer = viewer_over_rows(&mut tree);
    let surface = Size::new(100.0, 100.0);
    compute_layout(&mut tree, viewer, surface, &FixedAdvanceText::default()).expect("layout");
    let out = extract_clipped(&tree, viewer, surface, paint()).expect("extract");
    assert_eq!(out.scene.sprites.len(), 5);
    assert_eq!(out.clips.len(), 1);
    assert_eq!(out.clips[0].layers, 1..=4);
    assert_eq!(out.clips[0].rect.origin, [0.0, 0.0]);
    assert_eq!(out.clips[0].rect.size, [40.0, 40.0]);
}

#[test]
fn scrolled_content_outside_the_window_is_not_batched() {
    let mut tree = UiTree::new();
    let viewer = viewer_over_rows(&mut tree);
    tree.set(viewer, VERTICAL_OFFSET, 30.0).expect("test setup");
    let surface = Size::new(100.0, 100.0);
    compute_layout(&mut tree, viewer, surface, &FixedAdvanceText::default()).expect("layout");
    let out = extract_clipped(&tree, viewer, surface, paint()).expect("extract");
    let quads: usize = batch_scene2d_clipped(&out.scene, &out.clips)
        .iter()
        .map(SpriteBatch::quad_count)
        .sum();
    // The viewer's background plus the three rows still in the window; the
    // first row, scrolled out, is dropped.
    assert_eq!(quads, 4);
}

#[test]
fn a_tree_without_viewers_has_no_clips() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::StackPanel);
    filled(&mut tree, root, 10.0, 10.0);
    let surface = Size::new(100.0, 100.0);
    compute_layout(&mut tree, root, surface, &FixedAdvanceText::default()).expect("layout");
    let out = extract_clipped(&tree, root, surface, paint()).expect("extract");
    assert!(out.clips.is_empty());
}
