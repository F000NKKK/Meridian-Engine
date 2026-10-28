//! [`PackBuildError`]: why a pack could not be built.

use thiserror::Error;

use super::PackPath;
use crate::AssetId;

/// Why a pack could not be built.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PackBuildError {
    /// An asset id used twice.
    #[error("asset {0} is already in the pack")]
    DuplicateId(AssetId),
    /// A path used twice.
    #[error("`{0}` is already in the pack")]
    DuplicatePath(PackPath),
}
