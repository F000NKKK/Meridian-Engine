//! [`PackWriteError`]: why a pack could not be written.

use thiserror::Error;

use crate::pack::{PackBuildError, WriteAtError};

/// Why a streaming write failed. After any error the writer is not to be
/// used again: the header is written last, so what reached the sink is
/// never mistaken for a pack.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PackWriteError {
    /// The id or path is already in the pack.
    #[error(transparent)]
    Build(#[from] PackBuildError),
    /// The sink failed.
    #[error(transparent)]
    Sink(#[from] WriteAtError),
    /// An asset is still being written (`begin` without `end`).
    #[error("an asset is still open: end it first")]
    AssetOpen,
    /// There is no open asset to write to or end.
    #[error("no asset is open: begin one first")]
    NoAssetOpen,
    /// The pack would be larger than a file offset can say (2^64 bytes).
    #[error("the pack would be larger than 2^64 bytes")]
    TooLarge,
}
