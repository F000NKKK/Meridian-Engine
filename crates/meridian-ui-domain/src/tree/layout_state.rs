//! Layout results stored on nodes.

use super::UiTree;
use crate::{NodeId, Rect, Size, UiError};

impl UiTree {
    /// The node's rectangle from the last layout, relative to its parent's
    /// origin and already excluding its margin.
    pub fn layout_rect(&self, node: NodeId) -> Result<Rect, UiError> {
        Ok(self.element(node)?.rect)
    }

    /// The size the node asked for in the last measure pass, margin
    /// included.
    pub fn desired_size(&self, node: NodeId) -> Result<Size, UiError> {
        Ok(self.element(node)?.desired)
    }

    /// True when a structural or layout-affecting change happened since the
    /// last [`crate::compute_layout`].
    pub fn needs_layout(&self) -> bool {
        self.layout_dirty
    }

    pub(crate) fn set_desired(&mut self, node: NodeId, desired: Size) {
        if let Ok(element) = self.element_mut(node) {
            element.desired = desired;
        }
    }

    pub(crate) fn set_rect(&mut self, node: NodeId, rect: Rect) {
        if let Ok(element) = self.element_mut(node) {
            element.rect = rect;
        }
    }

    pub(crate) fn clear_layout_dirty(&mut self) {
        self.layout_dirty = false;
    }

    /// Children for layout; a stale node has none.
    pub(crate) fn children_vec(&self, node: NodeId) -> Vec<NodeId> {
        self.element(node)
            .map(|element| element.children.clone())
            .unwrap_or_default()
    }
}
