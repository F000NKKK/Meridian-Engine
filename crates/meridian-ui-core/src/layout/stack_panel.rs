//! `StackPanel`: children in a row or column.

use super::arrange::arrange as arrange_child;
use super::measure::measure as measure_child;
use super::text_measurer::TextMeasurer;
use crate::properties::stack_panel::ORIENTATION;
use crate::{NodeId, Point, Rect, Size, UiTree};

pub(super) fn measure(
    tree: &mut UiTree,
    text: &dyn TextMeasurer,
    node: NodeId,
    room: Size,
) -> Size {
    let orientation = tree.get_or_default(node, ORIENTATION);
    let offer = orientation.compose(f32::INFINITY, orientation.cross(room));
    let (mut main, mut cross) = (0.0_f32, 0.0_f32);
    for child in tree.children_vec(node) {
        let wanted = measure_child(tree, text, child, offer);
        main += orientation.main(wanted);
        cross = cross.max(orientation.cross(wanted));
    }
    orientation.compose(main, cross)
}

pub(super) fn arrange(tree: &mut UiTree, node: NodeId, size: Size) {
    let orientation = tree.get_or_default(node, ORIENTATION);
    let mut start = 0.0_f32;
    for child in tree.children_vec(node) {
        let step = orientation.main(tree.desired_size(child).unwrap_or_default());
        let extent = orientation.compose(step, orientation.cross(size));
        let at = orientation.compose(start, 0.0);
        let slot = Rect {
            origin: Point::new(at.width, at.height),
            size: extent,
        };
        arrange_child(tree, child, slot);
        start += step;
    }
}
