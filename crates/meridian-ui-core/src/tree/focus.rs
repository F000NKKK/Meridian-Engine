//! Keyboard focus.

use super::UiTree;
use crate::layout::value::Visibility;
use crate::properties::element::VISIBILITY;
use crate::properties::interaction::FOCUSABLE;
use crate::{NodeId, UiError, UiEvent, UiEventKind};

/// Which way focus moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    Next,
    Previous,
}

impl UiTree {
    /// The element holding keyboard focus.
    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    /// Gives `node` focus. Fails with [`UiError::NotFocusable`] unless it
    /// is `Focusable` and visible along its whole ancestor chain.
    pub fn focus(&mut self, node: NodeId) -> Result<(), UiError> {
        if !self.can_take_focus(node)? {
            return Err(UiError::NotFocusable(node));
        }
        self.set_focus(Some(node));
        Ok(())
    }

    /// Drops focus.
    pub fn clear_focus(&mut self) {
        self.set_focus(None);
    }

    /// Moves focus to the next or previous focusable element under `root`
    /// in document order, wrapping around. Returns the new holder, `None`
    /// when nothing under `root` can take focus.
    pub fn move_focus(&mut self, root: NodeId, direction: FocusDirection) -> Option<NodeId> {
        let order = self.focus_order(root);
        let position = self
            .focused
            .and_then(|node| order.iter().position(|candidate| *candidate == node));
        let target = match (direction, position) {
            (_, None) if order.is_empty() => return None,
            (FocusDirection::Next, None) => 0,
            (FocusDirection::Previous, None) => order.len() - 1,
            (FocusDirection::Next, Some(at)) => (at + 1) % order.len(),
            (FocusDirection::Previous, Some(at)) => (at + order.len() - 1) % order.len(),
        };
        self.set_focus(Some(order[target]));
        Some(order[target])
    }

    /// The focusable elements under `root`, in document order.
    pub(super) fn focus_order(&self, root: NodeId) -> Vec<NodeId> {
        let mut order = Vec::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if self.can_take_focus(node) == Ok(true) {
                order.push(node);
            }
            pending.extend(self.children_vec(node).into_iter().rev());
        }
        order
    }

    /// Focuses the nearest focusable ancestor-or-self of `node`, as a click
    /// does. Leaves focus alone when there is none.
    pub(super) fn focus_nearest(&mut self, node: NodeId) {
        let chain = std::iter::once(node).chain(self.ancestors(node));
        if let Some(target) = chain
            .into_iter()
            .find(|candidate| self.can_take_focus(*candidate) == Ok(true))
        {
            self.set_focus(Some(target));
        }
    }

    /// Focusable and visible all the way up.
    fn can_take_focus(&self, node: NodeId) -> Result<bool, UiError> {
        self.element(node)?;
        let chain = std::iter::once(node).chain(self.ancestors(node));
        let visible = chain
            .into_iter()
            .all(|id| self.get_or_default(id, VISIBILITY) == Visibility::Visible);
        Ok(visible && self.get_or_default(node, FOCUSABLE))
    }

    fn set_focus(&mut self, node: Option<NodeId>) {
        if self.focused == node {
            return;
        }
        if let Some(lost) = std::mem::replace(&mut self.focused, node) {
            self.events.push(UiEvent {
                target: lost,
                kind: UiEventKind::FocusLost,
            });
        }
        if let Some(gained) = node {
            self.events.push(UiEvent {
                target: gained,
                kind: UiEventKind::FocusGained,
            });
        }
    }
}
