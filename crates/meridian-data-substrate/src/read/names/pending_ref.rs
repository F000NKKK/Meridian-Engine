//! [`PendingRef`]: a reference to an entity a prefab is yet to make.

use substrate_data::Path;

use crate::EntityKey;

/// A reference to `<instance>/<something>`: an entity the prefab of
/// `instance` should make, which can only be checked once it is expanded.
#[derive(Debug, Clone)]
pub(crate) struct PendingRef {
    /// The entity named, as it will be keyed.
    pub(crate) key: EntityKey,
    /// The instance root it is under, as keyed.
    pub(crate) instance: EntityKey,
    /// Where the document names it.
    pub(crate) path: Path,
}
