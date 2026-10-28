//! [`PrefabLibrary`]: prefabs held in memory.

use std::collections::HashMap;

use super::{PrefabSource, PrefabUnavailable};
use crate::PrefabPath;

/// Prefabs held in memory, by path: a [`PrefabSource`] for tests, tools
/// that already have the bytes (an editor's open buffers), and a build
/// that read a directory once. A path it lacks is `PrefabUnavailable`,
/// with the closest path it has as the suggestion.
#[derive(Debug, Clone, Default)]
pub struct PrefabLibrary {
    prefabs: HashMap<PrefabPath, Vec<u8>>,
}

impl PrefabLibrary {
    /// An empty library.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds (or replaces) the prefab at `path`: `bytes` is an `.mprefab`
    /// in either form.
    pub fn insert(&mut self, path: PrefabPath, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.prefabs.insert(path, bytes.into());
        self
    }

    /// Whether it has a prefab at `path`.
    #[must_use]
    pub fn contains(&self, path: &PrefabPath) -> bool {
        self.prefabs.contains_key(path)
    }
}

impl PrefabSource for PrefabLibrary {
    fn prefab(&mut self, path: &PrefabPath) -> Result<Vec<u8>, PrefabUnavailable> {
        self.prefabs
            .get(path)
            .cloned()
            .ok_or_else(|| PrefabUnavailable {
                reason: "no such prefab".to_owned(),
                suggestion: path.closest(self.prefabs.keys()).cloned(),
            })
    }
}
