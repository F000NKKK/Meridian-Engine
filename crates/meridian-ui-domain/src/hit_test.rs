//! Finding the element under a point.

use crate::layout::value::Visibility;
use crate::properties::element::VISIBILITY;
use crate::properties::interaction::IS_HIT_TEST_VISIBLE;
use crate::{NodeId, Point, UiTree};

/// The topmost visible element under `point`, given in `root`'s parent
/// coordinates (for a root at the viewport origin, viewport pixels), using
/// the last layout, skipping elements that are not
/// [`IS_HIT_TEST_VISIBLE`] (their children still count). Later siblings are on top. An element clips its
/// children for hit testing: a child sticking out of its parent is not hit
/// outside the parent.
pub fn hit_test(tree: &UiTree, root: NodeId, point: Point) -> Option<NodeId> {
    let rect = tree.layout_rect(root).ok()?;
    let visibility: Visibility = tree.get_or_default(root, VISIBILITY);
    if visibility != Visibility::Visible || !rect.contains(point) {
        return None;
    }
    let local = Point::new(point.x - rect.origin.x, point.y - rect.origin.y);
    let children = tree.children(root).ok()?;
    let hit_child = children
        .iter()
        .rev()
        .find_map(|child| hit_test(tree, *child, local));
    let is_target: bool = tree.get_or_default(root, IS_HIT_TEST_VISIBLE);
    hit_child.or(is_target.then_some(root))
}
