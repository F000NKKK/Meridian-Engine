//! [`AssetMeta`]: what a `.meta` sidecar says about one asset.

use substrate_data::{Map, Value};

use super::document::MetaDocument;
use super::{ImporterName, MetaError, read};
use crate::AssetId;

/// An asset's sidecar metadata. Pure data: reading and writing it touches
/// no file, and it neither loads the asset nor decides how long it lives.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetMeta {
    /// The asset's stable identity.
    pub id: AssetId,
    /// Which importer reads the source file.
    pub importer: ImporterName,
    /// The importer's own settings, in file order. The importer reads them
    /// with its own schema; this crate does not know their keys.
    pub settings: Map,
    /// The assets this one needs loaded, each at most once, never itself.
    pub dependencies: Vec<AssetId>,
}

impl AssetMeta {
    /// Metadata with no settings and no dependencies.
    #[must_use]
    pub fn new(id: AssetId, importer: ImporterName) -> Self {
        AssetMeta {
            id,
            importer,
            settings: Map::new(),
            dependencies: Vec::new(),
        }
    }

    /// Reads a `.meta` file's `bytes`: the binary form when they start with
    /// `SDAT`, UTF-8 text otherwise (never decided by the extension).
    ///
    /// # Errors
    ///
    /// [`MetaError`] for bytes that do not decode, a `format` this does not
    /// read, fields that are missing, unknown or of the wrong type, and a
    /// dependency that repeats or names the asset itself.
    pub fn read(bytes: &[u8]) -> Result<Self, MetaError> {
        read::read(bytes)
    }

    /// The text form, as an editor saves it (always the newest `format`).
    #[must_use]
    pub fn to_text(&self) -> String {
        substrate_data::to_text(&MetaDocument::of(self))
    }

    /// The `SDAT` binary form, for cooked content (always the newest `format`).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        substrate_data::to_bytes(&MetaDocument::of(self))
    }

    /// One setting by key.
    #[must_use]
    pub fn setting(&self, key: &str) -> Option<&Value> {
        self.settings.get(key)
    }
}
