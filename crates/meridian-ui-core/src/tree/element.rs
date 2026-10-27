//! Per-node storage.

use crate::{ElementKind, NodeId, PropertyId, PropertyValue, Rect, Size};

/// One element: structure, explicitly set property values (the rest read
/// as defaults) and its last layout result.
#[derive(Debug)]
pub(super) struct Element {
    pub(super) kind: ElementKind,
    pub(super) parent: Option<NodeId>,
    pub(super) children: Vec<NodeId>,
    values: Vec<(PropertyId, PropertyValue)>,
    pub(super) desired: Size,
    pub(super) rect: Rect,
}

impl Element {
    pub(super) fn new(kind: ElementKind) -> Self {
        Self {
            kind,
            parent: None,
            children: Vec::new(),
            values: Vec::new(),
            desired: Size::ZERO,
            rect: Rect::default(),
        }
    }

    pub(super) fn value(&self, id: PropertyId) -> Option<&PropertyValue> {
        self.values
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, value)| value)
    }

    pub(super) fn put(&mut self, id: PropertyId, value: PropertyValue) {
        match self.values.iter_mut().find(|(key, _)| *key == id) {
            Some(slot) => slot.1 = value,
            None => self.values.push((id, value)),
        }
    }
}
