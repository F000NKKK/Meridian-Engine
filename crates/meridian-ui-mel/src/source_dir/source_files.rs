//! [`SourceFiles`]: the sources found under a directory.

use std::path::{Path, PathBuf};

use crate::BuildError;

/// The `.mel` and `.mres` files under a directory, each list sorted.
#[derive(Debug, Default)]
pub(crate) struct SourceFiles {
    pub mel: Vec<PathBuf>,
    pub mres: Vec<PathBuf>,
}

impl SourceFiles {
    /// Walks `dir` recursively.
    pub(crate) fn find(dir: &Path) -> Result<Self, BuildError> {
        let mut found = Self::default();
        found.walk(dir)?;
        found.mel.sort();
        found.mres.sort();
        Ok(found)
    }

    fn walk(&mut self, dir: &Path) -> Result<(), BuildError> {
        let entries = std::fs::read_dir(dir).map_err(|error| BuildError::io(dir, error))?;
        for entry in entries {
            let path = entry.map_err(|error| BuildError::io(dir, error))?.path();
            if path.is_dir() {
                self.walk(&path)?;
            } else if path.extension().is_some_and(|ext| ext == "mel") {
                self.mel.push(path);
            } else if path.extension().is_some_and(|ext| ext == "mres") {
                self.mres.push(path);
            }
        }
        Ok(())
    }
}
