//! Why a source could not give an asset's bytes.

use std::fmt;

use crate::AssetPath;

/// A [`ByteSource`](crate::ByteSource) failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// The source has no asset at this path.
    NotFound(AssetPath),
    /// The path is not allowed to name an asset (it escapes the source's root).
    Rejected(AssetPath),
    /// The asset exists but could not be read; `reason` is the system's text.
    Unreadable {
        /// The asset that failed.
        path: AssetPath,
        /// What the operating system or network said.
        reason: String,
    },
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(path) => write!(f, "asset not found: {path}"),
            Self::Rejected(path) => write!(f, "asset path rejected: {path}"),
            Self::Unreadable { path, reason } => write!(f, "cannot read asset {path}: {reason}"),
        }
    }
}

impl std::error::Error for SourceError {}
