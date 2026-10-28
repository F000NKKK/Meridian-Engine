//! [`Named`]: what a written entity name is.

use crate::EntityKey;

/// What a written entity name is.
pub(crate) enum Named {
    /// An entity of the document, as keyed.
    Local(EntityKey),
    /// An entity below an instance of the document, as keyed.
    Generated { key: EntityKey, instance: EntityKey },
}
