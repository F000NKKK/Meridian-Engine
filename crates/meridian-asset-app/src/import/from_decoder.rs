//! [`FromDecoder`]: a plain decoder as an importer.

use meridian_asset_domain::{AssetMeta, Decoder};

use super::{ImportFailure, Importer};

/// An [`Importer`] that is just a [`Decoder`]: the `.meta` settings are not
/// read. For an importer with settings, implement [`Importer`] directly.
#[derive(Debug, Clone, Copy, Default)]
pub struct FromDecoder<D>(pub D);

impl<T, D> Importer<T> for FromDecoder<D>
where
    D: Decoder<T> + Send + Sync,
    D::Error: std::error::Error + Send + Sync + 'static,
{
    fn import(&self, bytes: &[u8], _meta: &AssetMeta) -> Result<T, ImportFailure> {
        self.0.decode(bytes).map_err(|error| Box::new(error) as _)
    }
}
