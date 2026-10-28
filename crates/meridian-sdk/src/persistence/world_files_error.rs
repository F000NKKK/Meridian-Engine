//! [`WorldFilesError`]: why a scene, prefab or save file did not load or save.

use meridian_asset_app::SourceError;
use meridian_data_substrate::{LoadError, SaveError};
use thiserror::Error;

/// A failed [`WorldFiles`](super::WorldFiles) call. A failed load spawns
/// nothing.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorldFilesError {
    /// The file's bytes could not be read.
    #[error("{0}")]
    Read(#[from] SourceError),
    /// The background read never ran (the pool was cancelled or went away).
    #[error("the read of `{path}` did not run")]
    Interrupted {
        /// The file.
        path: String,
    },
    /// The scene, prefab or prefab reference does not load; every problem is
    /// in it, placed in the document.
    #[error("{0}")]
    Scene(#[from] LoadError),
    /// The save does not read.
    #[error("{0}")]
    Save(#[from] SaveError),
    /// The path may not be written: it is empty, absolute or leaves the root.
    #[error("`{path}` is not a path under the root")]
    PathRejected {
        /// The path as given.
        path: String,
    },
    /// The file could not be written.
    #[error("cannot write `{path}`: {source}")]
    Write {
        /// The file.
        path: String,
        /// What the operating system said.
        source: std::io::Error,
    },
}
