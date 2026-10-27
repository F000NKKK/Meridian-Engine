//! The tree arena.

use meridian_memory_core::ResourcePool;

use super::Change;
use super::element::Element;
use super::pointer_state::PointerState;
use crate::UiEvent;
use crate::{ElementKind, NodeId, UiError};

/// A retained element tree. Nodes live in a generational pool and refer to
/// each other by [`NodeId`]; there are no references and no reference
/// counting.
#[derive(Debug, Default)]
pub struct UiTree {
    pub(super) elements: ResourcePool<Element>,
    pub(super) changes: Vec<Change>,
    pub(super) layout_dirty: bool,
    pub(super) focused: Option<NodeId>,
    pub(super) pointer: PointerState,
    pub(super) events: Vec<UiEvent>,
}

impl UiTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of live nodes.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Creates a detached node.
    pub fn create(&mut self, kind: ElementKind) -> NodeId {
        self.layout_dirty = true;
        NodeId(self.elements.insert(Element::new(kind)))
    }

    pub fn contains(&self, node: NodeId) -> bool {
        self.elements.contains(node.0)
    }

    pub fn kind(&self, node: NodeId) -> Result<ElementKind, UiError> {
        Ok(self.element(node)?.kind)
    }

    pub fn parent(&self, node: NodeId) -> Result<Option<NodeId>, UiError> {
        Ok(self.element(node)?.parent)
    }

    pub fn children(&self, node: NodeId) -> Result<&[NodeId], UiError> {
        Ok(&self.element(node)?.children)
    }

    /// Takes and clears the recorded property changes, oldest first.
    pub fn drain_changes(&mut self) -> Vec<Change> {
        std::mem::take(&mut self.changes)
    }

    pub(super) fn element(&self, node: NodeId) -> Result<&Element, UiError> {
        self.elements.get(node.0).ok_or(UiError::StaleNode(node))
    }

    pub(super) fn element_mut(&mut self, node: NodeId) -> Result<&mut Element, UiError> {
        self.elements
            .get_mut(node.0)
            .ok_or(UiError::StaleNode(node))
    }
}
