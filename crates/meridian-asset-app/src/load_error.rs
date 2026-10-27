//! Why a background load produced no asset.

use std::fmt;

use crate::SourceError;

/// A failed [`AssetLoader::load`](crate::AssetLoader::load), generic over the
/// decoder's own error type `E`.
#[derive(Debug, PartialEq, Eq)]
pub enum LoadError<E> {
    /// The bytes could not be read.
    Source(SourceError),
    /// The bytes were read but the decoder rejected them.
    Decode(E),
    /// The decoder panicked; the panic stays on the background thread.
    Panicked,
    /// The job never ran: it was cancelled, or the background threads went away.
    Cancelled,
}

impl<E: fmt::Display> fmt::Display for LoadError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(f, "{error}"),
            Self::Decode(error) => write!(f, "decode failed: {error}"),
            Self::Panicked => f.write_str("the decoder panicked"),
            Self::Cancelled => f.write_str("the load was cancelled before it ran"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for LoadError<E> {}
