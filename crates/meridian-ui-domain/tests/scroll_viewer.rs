//! ScrollViewer layout, offsets and wheel input.

mod support;

use meridian_ui_domain::layout::value::Length;
use meridian_ui_domain::properties::element::HEIGHT;
use meridian_ui_domain::properties::scroll_viewer::VERTICAL_OFFSET;
use meridian_ui_domain::{ElementKind, InputEvent, InputOutcome, Point, Rect, Size, UiTree};
use support::{block, layout};

/// A 100x100 viewer over a 100x300 block.
fn tall_viewer(tree: &mut UiTree) -> (meridian_ui_domain::NodeId, meridian_ui_domain::NodeId) {
    let viewer = tree.create(ElementKind::ScrollViewer);
    let content = block(tree, viewer, 100.0, 300.0);
    (viewer, content)
}

#[test]
fn content_keeps_its_full_size_in_a_smaller_viewer() {
    let mut tree = UiTree::new();
    let (viewer, content) = tall_viewer(&mut tree);
    layout(&mut tree, viewer, 100.0, 100.0);
    assert_eq!(
        tree.layout_rect(viewer).unwrap().size,
        Size::new(100.0, 100.0)
    );
    assert_eq!(
        tree.layout_rect(content).unwrap(),
        Rect::new(0.0, 0.0, 100.0, 300.0)
    );
    assert_eq!(tree.scroll_extent(viewer).unwrap(), Size::new(100.0, 300.0));
}

#[test]
fn offset_moves_the_content_and_is_clamped() {
    let mut tree = UiTree::new();
    let (viewer, content) = tall_viewer(&mut tree);
    tree.set(viewer, VERTICAL_OFFSET, 50.0).unwrap();
    layout(&mut tree, viewer, 100.0, 100.0);
    assert_eq!(
        tree.layout_rect(content).unwrap().origin,
        Point::new(0.0, -50.0)
    );

    tree.set(viewer, VERTICAL_OFFSET, 9999.0).unwrap();
    layout(&mut tree, viewer, 100.0, 100.0);
    assert_eq!(
        tree.layout_rect(content).unwrap().origin,
        Point::new(0.0, -200.0)
    );
    assert_eq!(tree.scroll_offset(viewer).unwrap(), Point::new(0.0, 200.0));
}

#[test]
fn short_content_does_not_scroll() {
    let mut tree = UiTree::new();
    let viewer = tree.create(ElementKind::ScrollViewer);
    let content = block(&mut tree, viewer, 50.0, 50.0);
    tree.set(viewer, VERTICAL_OFFSET, 30.0).unwrap();
    layout(&mut tree, viewer, 100.0, 100.0);
    assert_eq!(
        tree.layout_rect(content).unwrap().origin,
        Point::new(0.0, 0.0)
    );
    assert!(!tree.scroll_by(viewer, Point::new(0.0, 10.0)).unwrap());
}

#[test]
fn scroll_by_reports_whether_it_moved() {
    let mut tree = UiTree::new();
    let (viewer, _) = tall_viewer(&mut tree);
    layout(&mut tree, viewer, 100.0, 100.0);
    assert!(tree.scroll_by(viewer, Point::new(0.0, 150.0)).unwrap());
    assert!(tree.scroll_by(viewer, Point::new(0.0, 150.0)).unwrap());
    assert!(!tree.scroll_by(viewer, Point::new(0.0, 150.0)).unwrap());
    layout(&mut tree, viewer, 100.0, 100.0);
    assert_eq!(tree.scroll_offset(viewer).unwrap().y, 200.0);
}

#[test]
fn wheel_scrolls_the_viewer_under_the_pointer() {
    let mut tree = UiTree::new();
    let (viewer, content) = tall_viewer(&mut tree);
    layout(&mut tree, viewer, 100.0, 100.0);
    let outcome = tree.handle_input(
        viewer,
        InputEvent::Scrolled {
            at: Point::new(10.0, 10.0),
            delta: Point::new(0.0, 40.0),
        },
    );
    assert_eq!(outcome, InputOutcome::Consumed);
    layout(&mut tree, viewer, 100.0, 100.0);
    assert_eq!(tree.layout_rect(content).unwrap().origin.y, -40.0);
}

#[test]
fn wheel_outside_the_viewer_is_ignored() {
    let mut tree = UiTree::new();
    let (viewer, _) = tall_viewer(&mut tree);
    layout(&mut tree, viewer, 100.0, 100.0);
    let outcome = tree.handle_input(
        viewer,
        InputEvent::Scrolled {
            at: Point::new(500.0, 10.0),
            delta: Point::new(0.0, 40.0),
        },
    );
    assert_eq!(outcome, InputOutcome::Ignored);
}

#[test]
fn wheel_passes_to_the_outer_viewer_at_the_inner_limit() {
    let mut tree = UiTree::new();
    let outer = tree.create(ElementKind::ScrollViewer);
    let stack = tree.create(ElementKind::StackPanel);
    tree.append_child(outer, stack).unwrap();
    let inner = tree.create(ElementKind::ScrollViewer);
    tree.append_child(stack, inner).unwrap();
    tree.set(inner, HEIGHT, Length::Px(100.0)).unwrap();
    block(&mut tree, inner, 100.0, 150.0);
    block(&mut tree, stack, 100.0, 400.0);
    layout(&mut tree, outer, 100.0, 100.0);
    // Inner viewer is 100x100 (clipped by the outer): scroll it to its end.
    let wheel = |tree: &mut UiTree| {
        let _ = tree.handle_input(
            outer,
            InputEvent::Scrolled {
                at: Point::new(10.0, 10.0),
                delta: Point::new(0.0, 100.0),
            },
        );
        layout(tree, outer, 100.0, 100.0);
    };
    wheel(&mut tree);
    wheel(&mut tree);
    assert_eq!(tree.scroll_offset(inner).unwrap().y, 50.0);
    assert!(tree.scroll_offset(outer).unwrap().y > 0.0);
}
