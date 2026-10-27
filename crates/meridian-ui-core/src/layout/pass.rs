//! Running layout over a tree.

use super::arrange::arrange;
use super::measure::measure;
use super::text_measurer::TextMeasurer;
use crate::{NodeId, Rect, Size, UiError, UiTree};

/// Lays out the subtree under `root` inside `viewport`: measures every
/// element, then arranges them, storing results readable through
/// [`UiTree::layout_rect`]. Runs unconditionally; callers use
/// [`UiTree::needs_layout`] to skip frames where nothing changed.
pub fn compute_layout(
    tree: &mut UiTree,
    root: NodeId,
    viewport: Size,
    text: &dyn TextMeasurer,
) -> Result<(), UiError> {
    tree.kind(root)?;
    measure(tree, text, root, viewport);
    arrange(tree, root, Rect::from_size(viewport));
    tree.clear_layout_dirty();
    Ok(())
}
