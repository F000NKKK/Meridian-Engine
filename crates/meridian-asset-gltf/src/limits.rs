//! Bounds that keep a hostile file from driving allocation.

/// Vertices across all parts of a model. A node tree can reference one
/// mesh many times, so this caps the product, not just each primitive.
pub(crate) const MAX_TOTAL_VERTICES: usize = 4_000_000;

/// Node nesting followed while flattening a scene.
pub(crate) const MAX_NODE_DEPTH: usize = 64;
