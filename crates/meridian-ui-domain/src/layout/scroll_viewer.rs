//! `ScrollViewer`: one child, shown through a window that can move over it.

use super::arrange::arrange as arrange_child;
use super::measure::measure as measure_child;
use super::text_measurer::TextMeasurer;
use crate::properties::scroll_viewer::{HORIZONTAL_OFFSET, VERTICAL_OFFSET};
use crate::{NodeId, Point, Rect, Size, UiTree};

pub(super) fn measure(
    tree: &mut UiTree,
    text: &dyn TextMeasurer,
    node: NodeId,
    room: Size,
) -> Size {
    let Some(child) = tree.children_vec(node).first().copied() else {
        return Size::ZERO;
    };
    let wanted = measure_child(tree, text, child, Size::UNBOUNDED);
    Size::new(wanted.width.min(room.width), wanted.height.min(room.height))
}

pub(super) fn arrange(tree: &mut UiTree, node: NodeId, size: Size) {
    let Some(child) = tree.children_vec(node).first().copied() else {
        return;
    };
    let extent = extent_for(tree, node, size);
    let offset = clamped_offset(tree, node, extent, size);
    let slot = Rect {
        origin: Point::new(-offset.x, -offset.y),
        size: extent,
    };
    arrange_child(tree, child, slot);
}

/// The size of the scrollable content for a viewport of `size`: what the
/// child wants, but at least the viewport.
fn extent_for(tree: &UiTree, node: NodeId, size: Size) -> Size {
    let wanted = tree
        .children(node)
        .ok()
        .and_then(|children| children.first())
        .and_then(|child| tree.desired_size(*child).ok())
        .unwrap_or_default();
    Size::new(wanted.width.max(size.width), wanted.height.max(size.height))
}

/// The stored offsets limited to the range the content allows.
fn clamped_offset(tree: &UiTree, node: NodeId, extent: Size, viewport: Size) -> Point {
    let limit =
        |offset: f32, content: f32, window: f32| offset.clamp(0.0, (content - window).max(0.0));
    Point::new(
        limit(
            tree.get_or_default(node, HORIZONTAL_OFFSET),
            extent.width,
            viewport.width,
        ),
        limit(
            tree.get_or_default(node, VERTICAL_OFFSET),
            extent.height,
            viewport.height,
        ),
    )
}

/// The size of a viewer's scrollable content after the last layout.
pub(crate) fn extent(tree: &UiTree, node: NodeId) -> Size {
    let viewport = tree
        .layout_rect(node)
        .map(|rect| rect.size)
        .unwrap_or_default();
    extent_for(tree, node, viewport)
}

/// Where a viewer's content is scrolled to after the last layout.
pub(crate) fn offset(tree: &UiTree, node: NodeId) -> Point {
    let viewport = tree
        .layout_rect(node)
        .map(|rect| rect.size)
        .unwrap_or_default();
    clamped_offset(tree, node, extent_for(tree, node, viewport), viewport)
}
