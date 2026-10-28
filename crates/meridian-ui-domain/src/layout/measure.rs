//! The measure pass.

use super::frame::Frame;
use super::text_measurer::TextMeasurer;
use super::{border, dock_panel, grid, scroll_viewer, stack_panel, text_block};
use crate::{ElementKind, NodeId, Size, UiTree};

/// Measures `node` given the room `available` (margin included) and stores
/// and returns its desired size, margin included.
pub(super) fn measure(
    tree: &mut UiTree,
    text: &dyn TextMeasurer,
    node: NodeId,
    available: Size,
) -> Size {
    let frame = Frame::read(tree, node);
    if frame.collapsed() {
        tree.set_desired(node, Size::ZERO);
        return Size::ZERO;
    }
    let room = frame.content_available(available);
    let content = measure_content(tree, text, node, room);
    let desired = frame.own_size(content).inflate(frame.margin);
    tree.set_desired(node, desired);
    desired
}

fn measure_content(tree: &mut UiTree, text: &dyn TextMeasurer, node: NodeId, room: Size) -> Size {
    match tree.kind(node) {
        Ok(ElementKind::Border) => border::measure(tree, text, node, room),
        Ok(ElementKind::StackPanel) => stack_panel::measure(tree, text, node, room),
        Ok(ElementKind::DockPanel) => dock_panel::measure(tree, text, node, room),
        Ok(ElementKind::Grid) => grid::measure(tree, text, node, room),
        Ok(ElementKind::ScrollViewer) => scroll_viewer::measure(tree, text, node, room),
        Ok(ElementKind::TextBlock) => text_block::measure(tree, text, node),
        _ => Size::ZERO,
    }
}
