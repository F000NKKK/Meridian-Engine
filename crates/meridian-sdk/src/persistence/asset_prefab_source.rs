//! [`AssetPrefabSource`]: prefab bytes from the asset layer.

use meridian_asset_app::{AssetLoader, AssetPath, LoadError, SourceError};
use meridian_data_substrate::{PrefabPath, PrefabSource, PrefabUnavailable};

use super::raw_bytes::RawBytes;

/// The extension a prefab file has; a [`PrefabPath`] is written without it.
const EXTENSION: &str = ".mprefab";

/// A [`PrefabSource`] over an [`AssetLoader`]: the prefab `actors/hero` is
/// the asset `actors/hero.mprefab` (a path that already ends in `.mprefab`
/// is used as written). The read runs on the loader's background threads
/// (ADR 027) and this call waits for it, so use it off the frame thread
/// and never from a job of the same pool; a prefab that is not there is
/// [`PrefabUnavailable`] at the entity that asked for it, as the data
/// layer reports it.
#[derive(Debug, Clone)]
pub struct AssetPrefabSource {
    loader: AssetLoader,
}

impl AssetPrefabSource {
    /// Prefabs read through `loader`.
    #[must_use]
    pub fn new(loader: AssetLoader) -> Self {
        Self { loader }
    }
}

/// The asset a prefab is stored as.
fn asset_of(path: &PrefabPath) -> AssetPath {
    let text = path.as_str();
    if text.ends_with(EXTENSION) {
        AssetPath::new(text)
    } else {
        AssetPath::new(format!("{text}{EXTENSION}"))
    }
}

impl PrefabSource for AssetPrefabSource {
    fn prefab(&mut self, path: &PrefabPath) -> Result<Vec<u8>, PrefabUnavailable> {
        match self.loader.load(asset_of(path), RawBytes).wait() {
            Ok(bytes) => Ok(bytes),
            Err(LoadError::Source(SourceError::NotFound(_))) => Err(PrefabUnavailable::missing()),
            Err(error) => Err(PrefabUnavailable {
                reason: error.to_string(),
                suggestion: None,
            }),
        }
    }
}
