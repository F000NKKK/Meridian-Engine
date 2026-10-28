//! [`PrefabSource`]: the port that gives a prefab's bytes.

use super::PrefabUnavailable;
use crate::PrefabPath;

/// Where prefabs come from. This crate has no filesystem and no asset
/// pipeline: the application implements this over its own (a directory,
/// a cooked bundle, an editor's open buffers) and passes it when it loads
/// a scene that instances prefabs. The bytes are an `.mprefab` in either
/// form (`SDAT` magic is binary, anything else text).
pub trait PrefabSource {
    /// The bytes of the prefab at `path`.
    ///
    /// # Errors
    ///
    /// [`PrefabUnavailable`] when there is no such prefab or it can't be
    /// read; put a `suggestion` in it when a close path exists.
    fn prefab(&mut self, path: &PrefabPath) -> Result<Vec<u8>, PrefabUnavailable>;
}
