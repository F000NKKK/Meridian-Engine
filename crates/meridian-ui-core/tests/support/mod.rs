//! Shared helpers for the layout tests.

use meridian_ui_core::layout::value::Length;
use meridian_ui_core::properties::element::{HEIGHT, WIDTH};
use meridian_ui_core::{ElementKind, FixedAdvanceText, NodeId, Size, UiTree, compute_layout};

/// A leaf of fixed pixel size under `parent`.
pub fn block(tree: &mut UiTree, parent: NodeId, width: f32, height: f32) -> NodeId {
    let node = tree.create(ElementKind::Border);
    tree.set(node, WIDTH, Length::Px(width)).unwrap();
    tree.set(node, HEIGHT, Length::Px(height)).unwrap();
    tree.append_child(parent, node).unwrap();
    node
}

/// Lays `root` out in a viewport with the deterministic text measurer.
pub fn layout(tree: &mut UiTree, root: NodeId, width: f32, height: f32) {
    compute_layout(
        tree,
        root,
        Size::new(width, height),
        &FixedAdvanceText::default(),
    )
    .unwrap();
}
