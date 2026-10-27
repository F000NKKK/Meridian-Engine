//! `Border`: one child inset by padding.

use super::arrange::arrange as arrange_child;
use super::measure::measure as measure_child;
use super::text_measurer::TextMeasurer;
use crate::properties::border::PADDING;
use crate::{NodeId, Rect, Size, UiTree};

pub(super) fn measure(
    tree: &mut UiTree,
    text: &dyn TextMeasurer,
    node: NodeId,
    room: Size,
) -> Size {
    let padding = tree.get_or_default(node, PADDING);
    let Some(child) = tree.children_vec(node).first().copied() else {
        return Size::ZERO.inflate(padding);
    };
    measure_child(tree, text, child, room.deflate(padding)).inflate(padding)
}

pub(super) fn arrange(tree: &mut UiTree, node: NodeId, size: Size) {
    let padding = tree.get_or_default(node, PADDING);
    if let Some(child) = tree.children_vec(node).first().copied() {
        arrange_child(tree, child, Rect::from_size(size).deflate(padding));
    }
}
