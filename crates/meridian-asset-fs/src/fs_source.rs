//! Asset bytes from a directory.

use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use meridian_asset_app::{AssetPath, ByteSource, SourceError};

/// A [`ByteSource`] that reads `root/<asset path>`.
#[derive(Debug, Clone)]
pub struct FsSource {
    root: PathBuf,
}

impl FsSource {
    /// A source rooted at `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The file for `path`, or `None` if the path could leave the root.
    fn resolve(&self, path: &AssetPath) -> Option<PathBuf> {
        let relative = Path::new(path.as_str());
        let stays_inside = relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
        stays_inside.then(|| self.root.join(relative))
    }
}

impl ByteSource for FsSource {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        let file = self
            .resolve(path)
            .ok_or_else(|| SourceError::Rejected(path.clone()))?;
        std::fs::read(file).map_err(|error| match error.kind() {
            ErrorKind::NotFound => SourceError::NotFound(path.clone()),
            _ => SourceError::Unreadable {
                path: path.clone(),
                reason: error.to_string(),
            },
        })
    }
}
