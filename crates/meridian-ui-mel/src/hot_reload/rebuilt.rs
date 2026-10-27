//! [`Rebuilt`]: a document built again after its shape changed.

use meridian_ui_domain::NodeId;

/// The root and the `x:Name`d elements of a rebuilt document. The typed
/// `Names` of the compiled module describe the old shape, so code-behind
/// reads the new ones from here, by the `Names` field name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rebuilt {
    pub root: NodeId,
    /// `Names` field and element of every `x:Name`, in document order.
    pub named: Vec<(String, NodeId)>,
}

impl Rebuilt {
    /// The element of `x:Name` field `field`.
    pub fn node(&self, field: &str) -> Option<NodeId> {
        self.named
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, node)| *node)
    }
}
