//! [`OpenError`]: why the content could not be opened.

use std::path::PathBuf;

use meridian_asset_domain::{PackError, ReadAtError};
use thiserror::Error;

/// Why [`Mounted::mount`](super::Mounted::mount) failed. Nothing is opened
/// then; the path says which layer.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum OpenError {
    /// A directory layer is not a directory.
    #[error("{} is not a directory", .0.display())]
    NotADirectory(PathBuf),
    /// A pack file could not be opened.
    #[error("cannot open pack {}: {error}", path.display())]
    File {
        /// The pack file.
        path: PathBuf,
        /// What the system said.
        error: ReadAtError,
    },
    /// A pack file is not a readable pack (damaged, truncated, newer).
    #[error("{} is not a readable pack: {error}", path.display())]
    Pack {
        /// The pack file.
        path: PathBuf,
        /// What is wrong with it.
        error: PackError,
    },
}
