//! Grid track sizing and placement.

mod support;

use meridian_ui_core::layout::value::GridLength::{Auto, Px, Star};
use meridian_ui_core::layout::value::{GridLength, Orientation};
use meridian_ui_core::properties::grid::{COLUMN, COLUMN_DEFINITIONS, ROW, ROW_DEFINITIONS};
use meridian_ui_core::properties::stack_panel::ORIENTATION;
use meridian_ui_core::{ElementKind, NodeId, Rect, UiTree};
use support::{block, layout};

fn grid(tree: &mut UiTree, rows: Vec<GridLength>, columns: Vec<GridLength>) -> NodeId {
    let node = tree.create(ElementKind::Grid);
    tree.set(node, ROW_DEFINITIONS, rows).unwrap();
    tree.set(node, COLUMN_DEFINITIONS, columns).unwrap();
    node
}

fn cell(tree: &mut UiTree, parent: NodeId, row: u32, column: u32, size: (f32, f32)) -> NodeId {
    let child = block(tree, parent, size.0, size.1);
    tree.set(child, ROW, row).unwrap();
    tree.set(child, COLUMN, column).unwrap();
    child
}

#[test]
fn pixel_auto_and_star_columns_share_the_width() {
    let mut tree = UiTree::new();
    let root = grid(
        &mut tree,
        vec![Star(1.0)],
        vec![Px(50.0), Auto, Star(1.0), Star(3.0)],
    );
    cell(&mut tree, root, 0, 1, (30.0, 10.0));
    let third = cell(&mut tree, root, 0, 2, (5.0, 5.0));
    let fourth = cell(&mut tree, root, 0, 3, (5.0, 5.0));
    layout(&mut tree, root, 280.0, 40.0);
    // 280 - 50 (px) - 30 (auto) = 200 shared 1:3 -> 50 and 150.
    assert_eq!(tree.layout_rect(third).unwrap().origin.x, 80.0);
    assert_eq!(tree.layout_rect(fourth).unwrap().origin.x, 130.0);
}

#[test]
fn star_cells_stretch_their_children_and_rows_work_the_same() {
    let mut tree = UiTree::new();
    let root = grid(&mut tree, vec![Px(20.0), Star(1.0)], vec![Star(1.0)]);
    let top = tree.create(ElementKind::Border);
    tree.append_child(root, top).unwrap();
    let bottom = tree.create(ElementKind::Border);
    tree.append_child(root, bottom).unwrap();
    tree.set(bottom, ROW, 1).unwrap();
    layout(&mut tree, root, 100.0, 100.0);
    assert_eq!(
        tree.layout_rect(top).unwrap(),
        Rect::new(0.0, 0.0, 100.0, 20.0)
    );
    assert_eq!(
        tree.layout_rect(bottom).unwrap(),
        Rect::new(0.0, 20.0, 100.0, 80.0)
    );
}

#[test]
fn no_definitions_means_a_single_star_cell() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::Grid);
    let child = tree.create(ElementKind::Border);
    tree.append_child(root, child).unwrap();
    layout(&mut tree, root, 64.0, 32.0);
    assert_eq!(
        tree.layout_rect(child).unwrap(),
        Rect::new(0.0, 0.0, 64.0, 32.0)
    );
}

#[test]
fn out_of_range_cells_clamp_to_the_last_track() {
    let mut tree = UiTree::new();
    let root = grid(&mut tree, vec![Star(1.0)], vec![Px(10.0), Px(10.0)]);
    let child = cell(&mut tree, root, 7, 9, (4.0, 4.0));
    layout(&mut tree, root, 100.0, 100.0);
    assert_eq!(tree.layout_rect(child).unwrap().origin.x, 10.0);
}

#[test]
fn star_tracks_size_to_content_when_the_room_is_unbounded() {
    let mut tree = UiTree::new();
    let outer = tree.create(ElementKind::StackPanel);
    tree.set(outer, ORIENTATION, Orientation::Horizontal)
        .unwrap();
    let root = grid(&mut tree, vec![Star(1.0)], vec![Star(1.0), Star(1.0)]);
    tree.append_child(outer, root).unwrap();
    cell(&mut tree, root, 0, 0, (30.0, 10.0));
    cell(&mut tree, root, 0, 1, (50.0, 10.0));
    layout(&mut tree, outer, f32::INFINITY, 40.0);
    // Measured with no width limit: columns are as wide as their content.
    assert_eq!(tree.desired_size(root).unwrap().width, 80.0);
}
