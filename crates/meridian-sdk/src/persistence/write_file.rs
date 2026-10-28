//! Writing a document under a root, all or nothing.

use std::fs;
use std::path::{Component, Path, PathBuf};

use super::WorldFilesError;

/// Writes `bytes` to `root/<path>`: through a sibling temporary file renamed
/// into place, so a crash leaves the old file or the new one, never half of
/// a save. Missing directories are created. `path` must stay inside `root`
/// (only plain components), the rule `FsSource` applies to reads.
pub(super) fn write_file(root: &Path, path: &str, bytes: &[u8]) -> Result<(), WorldFilesError> {
    let target = resolve(root, path)?;
    let write_error = |source| WorldFilesError::Write {
        path: path.to_owned(),
        source,
    };
    if let Some(directory) = target.parent() {
        fs::create_dir_all(directory).map_err(write_error)?;
    }
    let mut temporary = target.clone().into_os_string();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    fs::write(&temporary, bytes).map_err(write_error)?;
    fs::rename(&temporary, &target).map_err(write_error)
}

fn resolve(root: &Path, path: &str) -> Result<PathBuf, WorldFilesError> {
    let relative = Path::new(path);
    let plain = relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if path.is_empty() || !plain {
        return Err(WorldFilesError::PathRejected {
            path: path.to_owned(),
        });
    }
    Ok(root.join(relative))
}
