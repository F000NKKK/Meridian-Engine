//! [`SceneSaveError`]: why a scene could not be saved against its prefabs.

use thiserror::Error;

use crate::{EntityKey, LoadError, PrefabPath};

/// Why a scene could not be saved with its prefab instances written as
/// changes to their prefabs: an instance's prefab has to be read again to
/// see what changed. Nothing is written.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum SceneSaveError {
    /// The prefab of an instance is missing or does not load. The paths
    /// inside are those of loading the instance: `entities[0].prefab` for
    /// the prefab not being there, or paths in the prefab's own document.
    #[error("instance `{instance}` of prefab `{prefab}`: {error}")]
    Prefab {
        /// The instance.
        instance: EntityKey,
        /// Its prefab.
        prefab: PrefabPath,
        /// Why the prefab could not be read.
        error: LoadError,
    },
}
