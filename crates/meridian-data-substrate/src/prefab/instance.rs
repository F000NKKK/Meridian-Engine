//! [`PrefabInstance`]: a request to instance a prefab under a key.

use crate::{EntityKey, PrefabPath};

/// A prefab to instance and the key of the instance: the root of the
/// prefab becomes the entity `key`, the others `<key>/<prefab id>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabInstance {
    /// The instance's key: its root entity's.
    pub key: EntityKey,
    /// The prefab it is made from.
    pub prefab: PrefabPath,
}

impl PrefabInstance {
    /// An instance of `prefab` under `key`.
    #[must_use]
    pub fn new(key: EntityKey, prefab: PrefabPath) -> Self {
        PrefabInstance { key, prefab }
    }
}
