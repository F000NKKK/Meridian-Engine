//! Reading an asset's `.meta` through a source.

use meridian_asset_domain::AssetMeta;

use super::MetaReadError;
use crate::{AssetPath, ByteSource};

/// The sidecar of `asset` (`asset.sidecar()`), read whole from `source`.
/// Blocking, like the source.
///
/// # Errors
///
/// [`MetaReadError::Source`] when there is no sidecar or it cannot be read,
/// [`MetaReadError::Invalid`] when it is not a valid `.meta`.
pub fn read_meta(source: &dyn ByteSource, asset: &AssetPath) -> Result<AssetMeta, MetaReadError> {
    let path = asset.sidecar();
    let bytes = source.read(&path)?;
    AssetMeta::read(&bytes).map_err(|error| MetaReadError::Invalid { path, error })
}
