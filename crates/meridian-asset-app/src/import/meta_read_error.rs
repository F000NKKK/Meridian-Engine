//! [`MetaReadError`]: why an asset's `.meta` could not be read.

use meridian_asset_domain::MetaError;
use thiserror::Error;

use crate::{AssetPath, SourceError};

/// Why an asset's `.meta` could not be read.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum MetaReadError {
    /// The source could not give the file.
    #[error(transparent)]
    Source(#[from] SourceError),
    /// The file is not a valid `.meta`.
    #[error("{path}: {error}")]
    Invalid {
        /// The `.meta` file.
        path: AssetPath,
        /// What is wrong with it.
        #[source]
        error: MetaError,
    },
}
