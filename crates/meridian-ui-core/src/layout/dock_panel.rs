//! `DockPanel`: children docked to the edges of the remaining space.

use super::arrange::arrange as arrange_child;
use super::measure::measure as measure_child;
use super::text_measurer::TextMeasurer;
use super::value::Dock;
use crate::properties::dock_panel::{DOCK, LAST_CHILD_FILL};
use crate::{NodeId, Rect, Size, UiTree};

pub(super) fn measure(
    tree: &mut UiTree,
    text: &dyn TextMeasurer,
    node: NodeId,
    room: Size,
) -> Size {
    let children = tree.children_vec(node);
    let fill = tree.get_or_default(node, LAST_CHILD_FILL);
    let mut used = Size::ZERO;
    let mut total = Size::ZERO;
    for (index, child) in children.iter().enumerate() {
        let left = Size::new(
            (room.width - used.width).max(0.0),
            (room.height - used.height).max(0.0),
        );
        let wanted = measure_child(tree, text, *child, left);
        let fills = fill && index + 1 == children.len();
        match (fills, tree.get_or_default(*child, DOCK)) {
            (true, _) => grow_to_fit(&mut total, used, wanted),
            (false, Dock::Left | Dock::Right) => {
                total.height = total.height.max(used.height + wanted.height);
                used.width += wanted.width;
            }
            (false, Dock::Top | Dock::Bottom) => {
                total.width = total.width.max(used.width + wanted.width);
                used.height += wanted.height;
            }
        }
    }
    Size::new(total.width.max(used.width), total.height.max(used.height))
}

fn grow_to_fit(total: &mut Size, used: Size, wanted: Size) {
    total.width = total.width.max(used.width + wanted.width);
    total.height = total.height.max(used.height + wanted.height);
}

pub(super) fn arrange(tree: &mut UiTree, node: NodeId, size: Size) {
    let children = tree.children_vec(node);
    let fill = tree.get_or_default(node, LAST_CHILD_FILL);
    let mut remaining = Rect::from_size(size);
    for (index, child) in children.iter().enumerate() {
        let wanted = tree.desired_size(*child).unwrap_or_default();
        let fills = fill && index + 1 == children.len();
        let slot = if fills {
            remaining
        } else {
            dock_slot(&mut remaining, tree.get_or_default(*child, DOCK), wanted)
        };
        arrange_child(tree, *child, slot);
    }
}

/// Cuts `wanted` off the `dock` edge of `remaining` and returns the cut.
fn dock_slot(remaining: &mut Rect, dock: Dock, wanted: Size) -> Rect {
    let whole = *remaining;
    match dock {
        Dock::Left => {
            let width = wanted.width.min(whole.size.width);
            remaining.origin.x += width;
            remaining.size.width -= width;
            Rect::new(whole.origin.x, whole.origin.y, width, whole.size.height)
        }
        Dock::Top => {
            let height = wanted.height.min(whole.size.height);
            remaining.origin.y += height;
            remaining.size.height -= height;
            Rect::new(whole.origin.x, whole.origin.y, whole.size.width, height)
        }
        Dock::Right => {
            let width = wanted.width.min(whole.size.width);
            remaining.size.width -= width;
            Rect::new(
                whole.origin.x + whole.size.width - width,
                whole.origin.y,
                width,
                whole.size.height,
            )
        }
        Dock::Bottom => {
            let height = wanted.height.min(whole.size.height);
            remaining.size.height -= height;
            Rect::new(
                whole.origin.x,
                whole.origin.y + whole.size.height - height,
                whole.size.width,
                height,
            )
        }
    }
}
