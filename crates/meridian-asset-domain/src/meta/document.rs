//! The `.meta` document as Substrate Data reads and writes it.

use substrate_data::{Id, Map, Schema, Value};

use super::{ImporterName, META_FORMAT, MetaError};
use crate::AssetId;
use crate::asset_id::AssetKind;

/// A `.meta` file's fields, strictly: an unknown key is a problem, so a
/// file from a newer version is refused instead of losing data.
#[derive(Schema)]
pub(super) struct MetaDocument {
    pub(super) format: u32,
    pub(super) id: AssetId,
    pub(super) importer: ImporterName,
    /// A map, or absent.
    #[data(default)]
    pub(super) settings: Value,
    #[data(default)]
    pub(super) dependencies: Vec<Id<AssetKind>>,
}

impl MetaDocument {
    /// The document of `meta`, as written: every field present.
    pub(super) fn of(meta: &super::AssetMeta) -> Self {
        MetaDocument {
            format: META_FORMAT,
            id: meta.id,
            importer: meta.importer.clone(),
            settings: Value::Map(meta.settings.clone()),
            dependencies: meta.dependencies.iter().copied().map(Id::new).collect(),
        }
    }

    /// The settings as a map (`null`/absent is none), checked.
    pub(super) fn settings(&self) -> Result<Map, MetaError> {
        match &self.settings {
            Value::Null => Ok(Map::new()),
            Value::Map(map) => Ok(map.clone()),
            _ => Err(MetaError::SettingsNotAMap),
        }
    }

    /// The dependencies, without the asset itself or a repeat.
    pub(super) fn dependencies(&self) -> Result<Vec<AssetId>, MetaError> {
        let mut seen: Vec<AssetId> = Vec::with_capacity(self.dependencies.len());
        for (index, dependency) in self.dependencies.iter().enumerate() {
            let id = dependency.id;
            if id == self.id {
                return Err(MetaError::SelfDependency { id });
            }
            if let Some(first) = seen.iter().position(|&known| known == id) {
                return Err(MetaError::DuplicateDependency {
                    id,
                    first,
                    second: index,
                });
            }
            seen.push(id);
        }
        Ok(seen)
    }
}
