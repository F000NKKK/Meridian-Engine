//! [`SourceFiles`]: the sources found under a directory.

use std::path::{Path, PathBuf};

use crate::BuildError;

/// The `.mel` and `.mres` files under a directory, each list sorted
/// (culture variants last).
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
        // Default sets first: a variant is checked against its default.
        found
            .mres
            .sort_by_key(|path| (is_variant(path), path.clone()));
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

/// `Strings.ru.mres`: a stem with a culture after the set name.
fn is_variant(path: &Path) -> bool {
    path.file_stem()
        .is_some_and(|stem| stem.to_string_lossy().contains('.'))
}
