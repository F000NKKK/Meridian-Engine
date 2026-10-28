//! Why a mesh registry operation failed.

/// Why [`MeshRegistry::register`](crate::submission::MeshRegistry::register) refused a [`MeshSource`](crate::submission::MeshSource).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshRegistryError {
    /// An index names a vertex the mesh does not have.
    IndexOutOfRange {
        /// The offending index.
        index: u32,
        /// How many vertices the mesh has.
        vertex_count: usize,
    },
    /// `indices.len()` isn't a multiple of 3 — not a triangle list.
    NotATriangleList { index_count: usize },
    /// `uvs.len()` doesn't match `positions.len()` — every vertex needs
    /// exactly one UV, even on meshes only ever drawn unlit/untextured
    /// (the registry doesn't know a mesh's eventual material up front).
    UvCountMismatch { positions: usize, uvs: usize },
    /// `normals.len()` doesn't match `positions.len()`.
    NormalCountMismatch { positions: usize, normals: usize },
}

impl core::fmt::Display for MeshRegistryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MeshRegistryError::IndexOutOfRange {
                index,
                vertex_count,
            } => {
                write!(
                    f,
                    "index {index} is out of range for {vertex_count} vertices"
                )
            }
            MeshRegistryError::NotATriangleList { index_count } => {
                write!(f, "index count {index_count} is not a multiple of 3")
            }
            MeshRegistryError::UvCountMismatch { positions, uvs } => {
                write!(f, "{positions} positions but {uvs} uvs — must match")
            }
            MeshRegistryError::NormalCountMismatch { positions, normals } => {
                write!(
                    f,
                    "{positions} positions but {normals} normals — must match"
                )
            }
        }
    }
}

impl std::error::Error for MeshRegistryError {}

impl meridian_diagnostics::EngineError for MeshRegistryError {}
