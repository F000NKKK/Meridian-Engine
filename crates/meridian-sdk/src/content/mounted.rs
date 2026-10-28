//! [`Mounted`]: the opened layers, as one asset source.

use std::path::Path;
use std::sync::Arc;

use meridian_asset_app::{
    AssetCatalog, AssetListing, AssetLoader, ByteSource, CatalogError, OverlaySource, PackSource,
    Spawn,
};
use meridian_asset_fs::{FileReadAt, FsSource};
use meridian_runtime_threads::BackgroundPool;

use super::{Layer, Layers, OpenError};
use crate::asset_loader::default_threads;

/// The application's assets, opened from [`Layers`]: a directory, packs, or
/// a stack of them, behind the one [`ByteSource`] the loader and
/// `WorldFiles` read through. Opening reads each pack's header and index
/// only. Built once at startup by the composition root; nothing global.
///
/// Reading is all it does: saves are written by `WorldFiles` to the
/// directory it is given, which is a save directory, not a layer.
#[derive(Debug, Clone)]
pub struct Mounted {
    layers: Layers,
    source: Arc<OverlaySource>,
}

impl Mounted {
    /// Opens every layer of `layers`.
    ///
    /// # Errors
    ///
    /// [`OpenError`] when a directory is not one or a pack file does not
    /// open or is not a readable pack.
    pub fn mount(layers: impl Into<Layers>) -> Result<Self, OpenError> {
        let layers = layers.into();
        let opened = layers
            .layers()
            .iter()
            .map(open_layer)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Mounted {
            layers,
            source: Arc::new(OverlaySource::new(opened)),
        })
    }

    /// The layers this was opened from.
    #[must_use]
    pub fn layers(&self) -> &Layers {
        &self.layers
    }

    /// The assets as one byte source.
    #[must_use]
    pub fn source(&self) -> Arc<dyn ByteSource> {
        self.source.clone()
    }

    /// A loader reading this content on `spawner`'s threads (ADR 027).
    #[must_use]
    pub fn loader(&self, spawner: Arc<dyn Spawn>) -> AssetLoader {
        AssetLoader::new(self.source(), spawner)
    }

    /// A loader on a new background pool of `threads` threads.
    #[must_use]
    pub fn background_loader(&self, threads: usize) -> AssetLoader {
        self.loader(Arc::new(BackgroundPool::new(threads)))
    }

    /// [`background_loader`](Self::background_loader) with a pool sized as
    /// [`default_asset_loader`](crate::default_asset_loader) sizes it.
    #[must_use]
    pub fn default_loader(&self) -> AssetLoader {
        self.background_loader(default_threads())
    }

    /// The catalog of every `.meta` in the layers, with the top layer's
    /// version of an asset that several have. Blocking: off the frame thread.
    ///
    /// # Errors
    ///
    /// [`CatalogError`], as [`AssetCatalog::scan`].
    pub fn catalog(&self) -> Result<AssetCatalog, CatalogError> {
        AssetCatalog::scan(&*self.source)
    }
}

fn open_layer(layer: &Layer) -> Result<Arc<dyn AssetListing>, OpenError> {
    match layer {
        Layer::Directory(path) if path.is_dir() => Ok(Arc::new(FsSource::new(path))),
        Layer::Directory(path) => Err(OpenError::NotADirectory(path.clone())),
        Layer::Pack(path) => {
            open_pack(path).map(|pack| -> Arc<dyn AssetListing> { Arc::new(pack) })
        }
    }
}

fn open_pack(path: &Path) -> Result<PackSource<FileReadAt>, OpenError> {
    let file = FileReadAt::open(path).map_err(|error| OpenError::File {
        path: path.to_owned(),
        error,
    })?;
    PackSource::open(file).map_err(|error| OpenError::Pack {
        path: path.to_owned(),
        error,
    })
}
