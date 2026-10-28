//! [`RegisterError`]: why an importer could not be registered.

use meridian_asset_domain::ImporterName;
use thiserror::Error;

/// Why an importer could not be registered.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RegisterError {
    /// The name already has an importer; the first stays.
    #[error("an importer named `{0}` is already registered")]
    NameTaken(ImporterName),
}
