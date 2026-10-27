//! Attaching and removing nodes.

use super::UiTree;
use crate::{NodeId, UiError};

impl UiTree {
    /// Appends `child` as the last child of `parent`. Later children draw
    /// over earlier ones and win hit tests.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), UiError> {
        self.element(parent)?;
        if self.element(child)?.parent.is_some() {
            return Err(UiError::AlreadyAttached(child));
        }
        if self.is_self_or_ancestor(child, parent) {
            return Err(UiError::WouldCycle { parent, child });
        }
        self.element_mut(child)?.parent = Some(parent);
        self.element_mut(parent)?.children.push(child);
        self.layout_dirty = true;
        Ok(())
    }

    /// Removes `node` and its whole subtree. Their ids become stale.
    pub fn remove(&mut self, node: NodeId) -> Result<(), UiError> {
        if let Some(parent) = self.element(node)?.parent {
            self.element_mut(parent)?
                .children
                .retain(|child| *child != node);
        }
        let mut pending = vec![node];
        while let Some(current) = pending.pop() {
            if let Some(element) = self.elements.remove(current.0) {
                pending.extend(element.children);
            }
        }
        self.layout_dirty = true;
        Ok(())
    }

    /// True when `candidate` is `node` or one of its ancestors.
    fn is_self_or_ancestor(&self, candidate: NodeId, node: NodeId) -> bool {
        let mut current = Some(node);
        while let Some(id) = current {
            if id == candidate {
                return true;
            }
            current = self.element(id).ok().and_then(|element| element.parent);
        }
        false
    }
}
