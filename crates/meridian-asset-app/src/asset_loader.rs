//! Reads and decodes an asset on a background job.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use meridian_asset_domain::Decoder;

use crate::pending::{Completer, channel};
use crate::{AssetPath, ByteSource, LoadError, Pending, Spawn};

/// Loads assets without blocking the caller: [`load`](Self::load) returns at
/// once with a [`Pending`], and the read plus the decode run on the
/// [`Spawn`]er's threads. It decides nothing about where the decoded asset
/// lives afterwards; hand it to `meridian-resource`.
#[derive(Clone)]
pub struct AssetLoader {
    source: Arc<dyn ByteSource>,
    spawner: Arc<dyn Spawn>,
}

impl AssetLoader {
    /// A loader that reads from `source` and runs on `spawner`.
    #[must_use]
    pub fn new(source: Arc<dyn ByteSource>, spawner: Arc<dyn Spawn>) -> Self {
        Self { source, spawner }
    }

    /// Starts reading `path` and decoding it with `decoder`.
    #[must_use]
    pub fn load<T, D>(&self, path: AssetPath, decoder: D) -> Pending<Result<T, LoadError<D::Error>>>
    where
        T: Send + 'static,
        D: Decoder<T> + Send + 'static,
        D::Error: Send + 'static,
    {
        let (pending, completer) = channel(|| Err(LoadError::Cancelled));
        let source = Arc::clone(&self.source);
        self.spawner
            .spawn(Box::new(move || run(&*source, &path, &decoder, completer)));
        pending
    }
}

fn run<T, D: Decoder<T>>(
    source: &dyn ByteSource,
    path: &AssetPath,
    decoder: &D,
    completer: Completer<Result<T, LoadError<D::Error>>>,
) {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let bytes = source.read(path).map_err(LoadError::Source)?;
        decoder.decode(&bytes).map_err(LoadError::Decode)
    }));
    completer.complete(outcome.unwrap_or(Err(LoadError::Panicked)));
}

impl std::fmt::Debug for AssetLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetLoader").finish_non_exhaustive()
    }
}
