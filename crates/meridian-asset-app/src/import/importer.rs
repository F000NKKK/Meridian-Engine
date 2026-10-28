//! [`Importer`]: bytes and settings in, decoded content out.

use meridian_asset_domain::AssetMeta;

/// Why an importer rejected an asset: the importer's own error.
pub type ImportFailure = Box<dyn std::error::Error + Send + Sync>;

/// Turns the bytes of one asset into content of type `T`. The `.meta` is
/// passed so an importer can read its own `settings`; one that has none
/// (every plain [`Decoder`](meridian_asset_domain::Decoder), via
/// [`FromDecoder`](crate::FromDecoder)) ignores it. Pure over its inputs:
/// it reads no file and decides nothing about the content's lifetime.
pub trait Importer<T>: Send + Sync {
    /// The content in `bytes`, read as `meta` says.
    ///
    /// # Errors
    ///
    /// The importer's own error for bytes or settings it rejects. It must
    /// return an error, never panic, on malformed input.
    fn import(&self, bytes: &[u8], meta: &AssetMeta) -> Result<T, ImportFailure>;
}
