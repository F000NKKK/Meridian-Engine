//! The crate's error type.

use crate::{LiteralError, NodeId, PropertyId};

/// What can go wrong when editing or reading the element tree.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UiError {
    /// The node was removed, or its slot was reused by a newer node.
    #[error("stale node {0:?}")]
    StaleNode(NodeId),
    /// Attaching `child` under `parent` would make a node its own ancestor.
    #[error("attaching {child:?} under {parent:?} would create a cycle")]
    WouldCycle { parent: NodeId, child: NodeId },
    /// The node already has a parent; detach it first.
    #[error("node {0:?} already has a parent")]
    AlreadyAttached(NodeId),
    /// The node cannot take keyboard focus: it is not `Focusable`, or it
    /// or an ancestor is hidden or collapsed.
    #[error("node {0:?} cannot take focus")]
    NotFocusable(NodeId),
    /// No property with this id is registered in [`crate::properties`].
    #[error("unknown property {0:?}")]
    UnknownProperty(PropertyId),
    /// A value of the wrong kind was written to, or found in, a property.
    #[error("value of the wrong kind for property {0:?}")]
    ValueKind(PropertyId),
    /// Markup text that does not parse as the property's type.
    #[error("property {property:?}: {source}")]
    Literal {
        property: PropertyId,
        source: LiteralError,
    },
}

impl meridian_foundation::EngineError for UiError {}
