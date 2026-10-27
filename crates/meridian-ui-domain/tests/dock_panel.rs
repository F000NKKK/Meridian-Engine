//! DockPanel placement and hit testing on a docked layout.

mod support;

use meridian_ui_domain::layout::value::{Dock, Visibility};
use meridian_ui_domain::properties::dock_panel::{DOCK, LAST_CHILD_FILL};
use meridian_ui_domain::properties::element::VISIBILITY;
use meridian_ui_domain::{ElementKind, NodeId, Point, Rect, UiTree, hit_test};
use support::{block, layout};

fn docked(tree: &mut UiTree, root: NodeId, dock: Dock, size: (f32, f32)) -> NodeId {
    let node = block(tree, root, size.0, size.1);
    tree.set(node, DOCK, dock).unwrap();
    node
}

#[test]
fn edges_are_cut_in_order_and_the_last_child_fills() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::DockPanel);
    let top = docked(&mut tree, root, Dock::Top, (0.0, 20.0));
    let left = docked(&mut tree, root, Dock::Left, (30.0, 0.0));
    let bottom = docked(&mut tree, root, Dock::Bottom, (0.0, 10.0));
    let right = docked(&mut tree, root, Dock::Right, (15.0, 0.0));
    let fill = tree.create(ElementKind::Border);
    tree.append_child(root, fill).unwrap();
    layout(&mut tree, root, 200.0, 100.0);
    assert_eq!(
        tree.layout_rect(top).unwrap(),
        Rect::new(0.0, 0.0, 0.0, 20.0)
    );
    assert_eq!(tree.layout_rect(left).unwrap().origin.y, 20.0);
    assert_eq!(tree.layout_rect(bottom).unwrap().origin.y, 90.0);
    assert_eq!(tree.layout_rect(right).unwrap().origin.x, 185.0);
    assert_eq!(
        tree.layout_rect(fill).unwrap(),
        Rect::new(30.0, 20.0, 155.0, 70.0)
    );
}

#[test]
fn last_child_fill_can_be_turned_off() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::DockPanel);
    tree.set(root, LAST_CHILD_FILL, false).unwrap();
    let only = docked(&mut tree, root, Dock::Left, (30.0, 10.0));
    layout(&mut tree, root, 200.0, 100.0);
    assert_eq!(tree.layout_rect(only).unwrap().size.width, 30.0);
}

#[test]
fn hit_test_returns_the_topmost_visible_element() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::DockPanel);
    let left = docked(&mut tree, root, Dock::Left, (50.0, 100.0));
    let fill = tree.create(ElementKind::Border);
    tree.append_child(root, fill).unwrap();
    layout(&mut tree, root, 200.0, 100.0);
    assert_eq!(hit_test(&tree, root, Point::new(10.0, 10.0)), Some(left));
    assert_eq!(hit_test(&tree, root, Point::new(120.0, 10.0)), Some(fill));
    assert_eq!(hit_test(&tree, root, Point::new(500.0, 10.0)), None);
    tree.set(left, VISIBILITY, Visibility::Hidden).unwrap();
    assert_eq!(hit_test(&tree, root, Point::new(10.0, 10.0)), Some(root));
}

#[test]
fn later_siblings_win_overlaps() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::Grid);
    let below = tree.create(ElementKind::Border);
    let above = tree.create(ElementKind::Border);
    tree.append_child(root, below).unwrap();
    tree.append_child(root, above).unwrap();
    layout(&mut tree, root, 100.0, 100.0);
    assert_eq!(hit_test(&tree, root, Point::new(50.0, 50.0)), Some(above));
}
