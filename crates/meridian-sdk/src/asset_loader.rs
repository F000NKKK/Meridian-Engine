//! The default way an application builds its background asset loader.

use std::path::PathBuf;
use std::sync::Arc;

use meridian_asset_app::{AssetLoader, AssetPath, Pending};
use meridian_asset_domain::{DecodeError, ImageAsset};
use meridian_asset_fs::FsSource;
use meridian_platform_winit::CpuCapabilities;
use meridian_runtime_threads::BackgroundPool;

use crate::any_image_decoder::AnyImageDecoder;

/// An [`AssetLoader`] reading files under `root` on a [`BackgroundPool`] of
/// `threads` threads, kept apart from the frame workers (ADR 027): the
/// composition root's choice of adapters, so the loader crate names none.
#[must_use]
pub fn asset_loader(root: impl Into<PathBuf>, threads: usize) -> AssetLoader {
    AssetLoader::new(
        Arc::new(FsSource::new(root)),
        Arc::new(BackgroundPool::new(threads)),
    )
}

/// [`asset_loader`] with a pool sized to a quarter of the detected CPU
/// threads (at least one): asset reads wait on the disk, so they need few.
#[must_use]
pub fn default_asset_loader(root: impl Into<PathBuf>) -> AssetLoader {
    let threads = (CpuCapabilities::detect().threads / 4).max(1);
    asset_loader(root, threads)
}

/// Starts loading the image at `path` (BMP or PNG, found by magic bytes)
/// and returns at once; poll the [`Pending`] or `wait` on it.
#[must_use]
pub fn load_image(
    loader: &AssetLoader,
    path: &str,
) -> Pending<Result<ImageAsset, meridian_asset_app::LoadError<DecodeError>>> {
    loader.load(AssetPath::new(path), AnyImageDecoder)
}
