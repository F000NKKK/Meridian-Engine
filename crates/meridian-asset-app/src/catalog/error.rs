//! [`CatalogError`]: why a catalog could not be built.

use meridian_asset_domain::AssetId;
use thiserror::Error;

use crate::{AssetPath, MetaReadError, SourceError};

/// Why a catalog could not be built. A catalog is built whole or not at all.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum CatalogError {
    /// The source could not be walked.
    #[error("cannot list the assets: {0}")]
    Listing(#[source] SourceError),
    /// A `.meta` could not be read or is not valid.
    #[error(transparent)]
    Meta(#[from] MetaReadError),
    /// Two assets claim the same id.
    #[error("asset {id} is claimed by both {first} and {second}")]
    DuplicateId {
        /// The id.
        id: AssetId,
        /// The asset seen first (paths in sorted order).
        first: AssetPath,
        /// The other.
        second: AssetPath,
    },
    /// An asset depends on an id no `.meta` has.
    #[error("{asset} depends on {dependency}, which no .meta declares")]
    MissingDependency {
        /// The asset that needs it.
        asset: AssetPath,
        /// The id it names.
        dependency: AssetId,
    },
    /// Assets depend on each other in a loop.
    #[error("assets depend on each other in a loop through {asset}")]
    DependencyCycle {
        /// An asset on the loop.
        asset: AssetPath,
    },
}
