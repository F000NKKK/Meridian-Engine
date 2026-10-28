//! The name of an asset inside a source.

use std::fmt;

/// Where an asset is, relative to its source's root, with `/` separators
/// (`"textures/grass.png"`). A distinct type so a path is never confused with
/// asset text; how it maps to a file or URL is the [`ByteSource`](crate::ByteSource)'s job.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath(String);

impl AssetPath {
    /// A path from its `/`-separated text.
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// The path of this asset's `.meta` sidecar: the same path with `.meta`
    /// appended (`meshes/crate.obj` has `meshes/crate.obj.meta`).
    #[must_use]
    pub fn sidecar(&self) -> AssetPath {
        AssetPath(format!("{}.meta", self.0))
    }

    /// The `/`-separated text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
