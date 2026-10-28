//! [`DocumentRole`]: what a document being read is for.

use crate::EntityKey;
use crate::document_keys::{DOCUMENT, PREFAB_DOCUMENT};

/// What a document being read is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocumentRole {
    /// A scene: ids are its own.
    Scene,
    /// A prefab read as itself (an editor opening it): ids are local.
    Prefab,
    /// A prefab instanced under this key: its root is that entity and the
    /// others are named `<instance>/<prefab id>`.
    Instance(EntityKey),
}

impl DocumentRole {
    /// The keys the document may have.
    pub(crate) fn keys(&self) -> &'static [&'static str] {
        match self {
            DocumentRole::Scene => &DOCUMENT,
            DocumentRole::Prefab | DocumentRole::Instance(_) => &PREFAB_DOCUMENT,
        }
    }

    /// Whether it is a prefab (read as itself or instanced).
    pub(crate) fn is_prefab(&self) -> bool {
        !matches!(self, DocumentRole::Scene)
    }

    /// The key an entity written as `local` has once read: itself, or its
    /// name under the instance (the root being the instance itself).
    pub(crate) fn key_of(&self, local: EntityKey, is_root: bool) -> EntityKey {
        match self {
            DocumentRole::Instance(instance) if is_root => instance.clone(),
            DocumentRole::Instance(instance) => instance.under(&local),
            DocumentRole::Scene | DocumentRole::Prefab => local,
        }
    }
}
