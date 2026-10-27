//! Why a mesh registry operation failed.

/// Why [`MeshRegistry::register`](crate::submission::MeshRegistry::register) refused a [`MeshSource`](crate::submission::MeshSource).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshRegistryError {
    /// More vertices than a `u16` index can address (see [`MeshSource`](crate::submission::MeshSource)'s
    /// doc comment).
    TooManyVertices { count: usize },
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
            MeshRegistryError::TooManyVertices { count } => {
                write!(
                    f,
                    "mesh has {count} vertices, beyond u16 index range (65535)"
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

impl meridian_foundation::EngineError for MeshRegistryError {}
