//! `TextBlock` measuring.

use super::text_measurer::TextMeasurer;
use crate::properties::text_block::{FONT_SIZE, TEXT};
use crate::{NodeId, Size, UiTree};

pub(super) fn measure(tree: &UiTree, text: &dyn TextMeasurer, node: NodeId) -> Size {
    let content: String = tree.get_or_default(node, TEXT);
    text.measure(&content, tree.get_or_default(node, FONT_SIZE))
}
