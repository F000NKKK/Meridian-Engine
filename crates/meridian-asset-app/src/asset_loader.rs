//! Reads and decodes an asset on a background job.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use meridian_asset_domain::Decoder;

use crate::pending::{Completer, channel};
use crate::{AssetPath, ByteSource, LoadError, Pending, Priority, Spawn};

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

    /// Starts reading `path` and decoding it with `decoder`, at
    /// [`Priority::Normal`].
    #[must_use]
    pub fn load<T, D>(&self, path: AssetPath, decoder: D) -> Pending<Result<T, LoadError<D::Error>>>
    where
        T: Send + 'static,
        D: Decoder<T> + Send + 'static,
        D::Error: Send + 'static,
    {
        self.load_with(Priority::Normal, path, decoder)
    }

    /// [`load`](Self::load) with an explicit `priority`. Call
    /// [`Pending::cancel`] on the result to drop the load if it is no longer
    /// wanted before it starts.
    #[must_use]
    pub fn load_with<T, D>(
        &self,
        priority: Priority,
        path: AssetPath,
        decoder: D,
    ) -> Pending<Result<T, LoadError<D::Error>>>
    where
        T: Send + 'static,
        D: Decoder<T> + Send + 'static,
        D::Error: Send + 'static,
    {
        self.submit(priority, move |source| {
            let bytes = source.read(&path).map_err(LoadError::Source)?;
            decoder.decode(&bytes).map_err(LoadError::Decode)
        })
    }

    /// Runs `work` on a background job with the loader's source, with the
    /// loader's guarantees: the result always arrives, a panic becomes
    /// [`LoadError::Panicked`], a job never run becomes [`LoadError::Cancelled`].
    pub(crate) fn submit<T, E, F>(
        &self,
        priority: Priority,
        work: F,
    ) -> Pending<Result<T, LoadError<E>>>
    where
        T: Send + 'static,
        E: Send + 'static,
        F: FnOnce(&dyn ByteSource) -> Result<T, LoadError<E>> + Send + 'static,
    {
        let (pending, completer) = channel(|| Err(LoadError::Cancelled));
        let source = Arc::clone(&self.source);
        self.spawner
            .spawn(priority, Box::new(move || run(&*source, work, completer)));
        pending
    }
}

fn run<T, E, F>(source: &dyn ByteSource, work: F, completer: Completer<Result<T, LoadError<E>>>)
where
    F: FnOnce(&dyn ByteSource) -> Result<T, LoadError<E>>,
{
    if completer.is_cancelled() {
        return completer.complete(Err(LoadError::Cancelled));
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| work(source)));
    completer.complete(outcome.unwrap_or(Err(LoadError::Panicked)));
}

impl std::fmt::Debug for AssetLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetLoader").finish_non_exhaustive()
    }
}
