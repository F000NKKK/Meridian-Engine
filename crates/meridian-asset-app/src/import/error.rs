//! [`ImportError`]: why an asset could not be imported.

use meridian_asset_domain::{ImporterName, MetaError};
use thiserror::Error;

use super::ImportFailure;
use crate::AssetPath;

/// Why an asset could not be imported.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ImportError {
    /// The `.meta` names an importer the registry does not have.
    #[error("no importer named `{0}` is registered")]
    UnknownImporter(ImporterName),
    /// The importer rejected the asset.
    #[error("importer `{importer}` failed: {source}")]
    Failed {
        /// The importer that was asked.
        importer: ImporterName,
        /// What it said.
        #[source]
        source: ImportFailure,
    },
    /// The asset's `.meta` file is not valid.
    #[error("{path}: {error}")]
    InvalidMeta {
        /// The `.meta` file.
        path: AssetPath,
        /// What is wrong with it.
        #[source]
        error: MetaError,
    },
}
