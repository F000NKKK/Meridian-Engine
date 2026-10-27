//! StackPanel, Border, margins, alignment and text sizing.

mod support;

use meridian_ui_domain::layout::value::{Alignment, Length, Orientation, Visibility};
use meridian_ui_domain::properties::border::PADDING;
use meridian_ui_domain::properties::element::{
    HORIZONTAL_ALIGNMENT, MARGIN, VERTICAL_ALIGNMENT, VISIBILITY, WIDTH,
};
use meridian_ui_domain::properties::stack_panel::ORIENTATION;
use meridian_ui_domain::properties::text_block::{FONT_SIZE, TEXT};
use meridian_ui_domain::{ElementKind, Rect, Thickness, UiTree};
use support::{block, layout};

#[test]
fn vertical_stack_sums_heights_and_stretches_width() {
    let mut tree = UiTree::new();
    let stack = tree.create(ElementKind::StackPanel);
    let first = block(&mut tree, stack, 30.0, 10.0);
    let second = block(&mut tree, stack, 50.0, 20.0);
    layout(&mut tree, stack, 200.0, 100.0);
    assert_eq!(tree.desired_size(stack).unwrap().height, 30.0);
    assert_eq!(
        tree.layout_rect(first).unwrap(),
        Rect::new(0.0, 0.0, 30.0, 10.0)
    );
    assert_eq!(
        tree.layout_rect(second).unwrap(),
        Rect::new(0.0, 10.0, 50.0, 20.0)
    );
}

#[test]
fn horizontal_stack_places_children_left_to_right() {
    let mut tree = UiTree::new();
    let stack = tree.create(ElementKind::StackPanel);
    tree.set(stack, ORIENTATION, Orientation::Horizontal)
        .unwrap();
    block(&mut tree, stack, 30.0, 10.0);
    let second = block(&mut tree, stack, 50.0, 20.0);
    layout(&mut tree, stack, 200.0, 100.0);
    assert_eq!(
        tree.layout_rect(second).unwrap(),
        Rect::new(30.0, 0.0, 50.0, 20.0)
    );
}

#[test]
fn margin_adds_to_the_slot_and_offsets_the_element() {
    let mut tree = UiTree::new();
    let stack = tree.create(ElementKind::StackPanel);
    let first = block(&mut tree, stack, 30.0, 10.0);
    tree.set(first, MARGIN, Thickness::new(4.0, 2.0, 6.0, 3.0))
        .unwrap();
    let second = block(&mut tree, stack, 30.0, 10.0);
    layout(&mut tree, stack, 200.0, 100.0);
    assert_eq!(
        tree.layout_rect(first).unwrap(),
        Rect::new(4.0, 2.0, 30.0, 10.0)
    );
    assert_eq!(tree.layout_rect(second).unwrap().origin.y, 15.0);
}

#[test]
fn alignment_places_a_fixed_size_child_in_its_slot() {
    let mut tree = UiTree::new();
    let stack = tree.create(ElementKind::StackPanel);
    let child = block(&mut tree, stack, 40.0, 10.0);
    tree.set(child, HORIZONTAL_ALIGNMENT, Alignment::Center)
        .unwrap();
    layout(&mut tree, stack, 100.0, 100.0);
    assert_eq!(
        tree.layout_rect(child).unwrap(),
        Rect::new(30.0, 0.0, 40.0, 10.0)
    );
    tree.set(child, HORIZONTAL_ALIGNMENT, Alignment::End)
        .unwrap();
    layout(&mut tree, stack, 100.0, 100.0);
    assert_eq!(tree.layout_rect(child).unwrap().origin.x, 60.0);
}

#[test]
fn auto_sized_child_stretches_across_the_stack() {
    let mut tree = UiTree::new();
    let stack = tree.create(ElementKind::StackPanel);
    let child = tree.create(ElementKind::Border);
    tree.append_child(stack, child).unwrap();
    layout(&mut tree, stack, 100.0, 100.0);
    assert_eq!(tree.layout_rect(child).unwrap().size.width, 100.0);
    tree.set(child, HORIZONTAL_ALIGNMENT, Alignment::Start)
        .unwrap();
    layout(&mut tree, stack, 100.0, 100.0);
    assert_eq!(tree.layout_rect(child).unwrap().size.width, 0.0);
}

#[test]
fn collapsed_children_take_no_space_and_hidden_ones_do() {
    let mut tree = UiTree::new();
    let stack = tree.create(ElementKind::StackPanel);
    let collapsed = block(&mut tree, stack, 30.0, 10.0);
    let hidden = block(&mut tree, stack, 30.0, 10.0);
    let last = block(&mut tree, stack, 30.0, 10.0);
    tree.set(collapsed, VISIBILITY, Visibility::Collapsed)
        .unwrap();
    tree.set(hidden, VISIBILITY, Visibility::Hidden).unwrap();
    layout(&mut tree, stack, 100.0, 100.0);
    assert_eq!(tree.layout_rect(last).unwrap().origin.y, 10.0);
    assert_eq!(tree.layout_rect(collapsed).unwrap(), Rect::default());
}

#[test]
fn border_insets_its_child_by_padding() {
    let mut tree = UiTree::new();
    let border = tree.create(ElementKind::Border);
    tree.set(border, PADDING, Thickness::uniform(5.0)).unwrap();
    let child = block(&mut tree, border, 20.0, 10.0);
    tree.set(child, HORIZONTAL_ALIGNMENT, Alignment::Start)
        .unwrap();
    tree.set(child, VERTICAL_ALIGNMENT, Alignment::Start)
        .unwrap();
    layout(&mut tree, border, 100.0, 100.0);
    assert_eq!(tree.desired_size(border).unwrap().width, 30.0);
    assert_eq!(
        tree.layout_rect(child).unwrap(),
        Rect::new(5.0, 5.0, 20.0, 10.0)
    );
}

#[test]
fn text_block_is_sized_by_the_measurer() {
    let mut tree = UiTree::new();
    let stack = tree.create(ElementKind::StackPanel);
    let label = tree.create(ElementKind::TextBlock);
    tree.append_child(stack, label).unwrap();
    tree.set(label, TEXT, "Hello".to_string()).unwrap();
    tree.set(label, FONT_SIZE, 20.0).unwrap();
    layout(&mut tree, stack, 400.0, 400.0);
    assert_eq!(tree.desired_size(label).unwrap().width, 50.0);
    assert_eq!(tree.desired_size(label).unwrap().height, 20.0);
}

#[test]
fn explicit_size_wins_over_available_room() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::Border);
    tree.set(root, WIDTH, Length::Px(500.0)).unwrap();
    layout(&mut tree, root, 100.0, 100.0);
    assert_eq!(tree.layout_rect(root).unwrap().size.width, 500.0);
}
