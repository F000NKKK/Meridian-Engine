//! Bounds that keep a hostile file from driving allocation.

/// Vertices across all parts of a model. A node tree can reference one
/// mesh many times, so this caps the product, not just each primitive.
pub(crate) const MAX_TOTAL_VERTICES: usize = 4_000_000;

/// Joints of the skeleton that holds the nodes animation moves; a model
/// needing more keeps those nodes static.
pub(crate) const MAX_MOTION_JOINTS: usize = 256;

/// Node nesting followed while flattening a scene.
pub(crate) const MAX_NODE_DEPTH: usize = 64;
