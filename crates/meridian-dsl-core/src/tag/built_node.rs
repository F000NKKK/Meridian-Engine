//! [`BuiltNode`]: one node of a tree built by [`crate::DslRegistry`].

use std::any::Any;
use std::fmt;

/// One built node of a parsed DSL tree: the source tag name, the typed
/// value a registered [`crate::DslTag`] impl produced from its attributes (as
/// `Box<dyn Any>` — the registry itself has no compile-time knowledge
/// of which concrete type a given tag name maps to; a caller downcasts
/// by tag name, the same pattern `ecs-core`'s type-erased component
/// storage uses for the same reason), and its children, recursively
/// built the same way.
pub struct BuiltNode {
    pub tag: String,
    pub value: Box<dyn Any>,
    pub children: Vec<BuiltNode>,
}

impl fmt::Debug for BuiltNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuiltNode")
            .field("tag", &self.tag)
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

impl BuiltNode {
    /// Downcasts [`value`](Self::value) to `T`, if this node's tag was
    /// built by `T`'s [`crate::DslTag`] impl. `None` on a mismatch — a caller
    /// walking a tree of mixed tag types checks `node.tag` first, then
    /// calls this for the type it expects.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }
}
