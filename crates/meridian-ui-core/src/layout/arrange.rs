//! The arrange pass.

use super::value::Alignment;
use super::{border, dock_panel, frame::Frame, grid, stack_panel};
use crate::{ElementKind, NodeId, Point, Rect, Size, UiTree};

/// Places `node` inside `slot` (in its parent's coordinates, margin
/// included), stores its rectangle and arranges its children.
pub(super) fn arrange(tree: &mut UiTree, node: NodeId, slot: Rect) {
    let frame = Frame::read(tree, node);
    if frame.collapsed() {
        tree.set_rect(node, Rect::default());
        return;
    }
    let room = slot.size.deflate(frame.margin);
    let wanted = tree
        .desired_size(node)
        .unwrap_or_default()
        .deflate(frame.margin);
    let width = extent(
        frame.width_is_explicit(),
        frame.horizontal,
        wanted.width,
        room.width,
    );
    let height = extent(
        frame.height_is_explicit(),
        frame.vertical,
        wanted.height,
        room.height,
    );
    let x = slot.origin.x + frame.margin.left + offset(frame.horizontal, room.width - width);
    let y = slot.origin.y + frame.margin.top + offset(frame.vertical, room.height - height);
    tree.set_rect(
        node,
        Rect {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        },
    );
    arrange_content(tree, node, Size::new(width, height));
}

/// The final extent on one axis. An explicit size is kept as is.
fn extent(explicit: bool, alignment: Alignment, wanted: f32, room: f32) -> f32 {
    if explicit {
        return wanted;
    }
    match alignment {
        Alignment::Stretch => room,
        _ => wanted.min(room),
    }
}

/// How far into the leftover space the element starts.
fn offset(alignment: Alignment, leftover: f32) -> f32 {
    match alignment {
        Alignment::Start | Alignment::Stretch => 0.0,
        Alignment::Center => leftover / 2.0,
        Alignment::End => leftover,
    }
}

fn arrange_content(tree: &mut UiTree, node: NodeId, size: Size) {
    match tree.kind(node) {
        Ok(ElementKind::Border) => border::arrange(tree, node, size),
        Ok(ElementKind::StackPanel) => stack_panel::arrange(tree, node, size),
        Ok(ElementKind::DockPanel) => dock_panel::arrange(tree, node, size),
        Ok(ElementKind::Grid) => grid::arrange(tree, node, size),
        _ => {}
    }
}
