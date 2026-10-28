//! [`ComponentsDraft`]: an entity's components as read, not inserted yet.

use crate::registry::Insert;
use crate::{ComponentName, KeptComponent};

/// An entity's components as read: registered ones waiting for their
/// entity, unknown ones kept as written. One per name.
#[derive(Default)]
pub(crate) struct ComponentsDraft {
    pub(crate) inserts: Vec<(ComponentName, Insert)>,
    pub(crate) kept: Vec<KeptComponent>,
}

impl ComponentsDraft {
    /// These components with `over`'s on top: a component `over` has
    /// replaces the whole of the one of that name here (field-level
    /// overrides are a later format bump).
    pub(crate) fn overridden_by(mut self, over: ComponentsDraft) -> ComponentsDraft {
        let replaced = |name: &ComponentName| {
            over.inserts.iter().any(|(n, _)| n == name) || over.kept.iter().any(|k| &k.name == name)
        };
        self.inserts.retain(|(name, _)| !replaced(name));
        self.kept.retain(|k| !replaced(&k.name));
        self.inserts.extend(over.inserts);
        self.kept.extend(over.kept);
        self
    }
}
