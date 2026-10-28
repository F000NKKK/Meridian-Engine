//! How often a vertex buffer advances.

/// Whether a vertex buffer advances once per vertex or once per instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VertexStep {
    /// One element per vertex: mesh data.
    #[default]
    Vertex,
    /// One element per instance: per-draw data such as a transform.
    Instance,
}
