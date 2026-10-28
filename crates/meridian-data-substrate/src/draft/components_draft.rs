//! [`ComponentsDraft`]: an entity's components as read, not inserted yet.

use substrate_data::Value;

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
    /// replaces the whole of the one of that name here (an instance's
    /// `overrides` change single fields, applied after this).
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

    /// Component `name` as its document value, whichever way it is held.
    pub(crate) fn document(&self, name: &ComponentName) -> Option<Value> {
        let read = self.inserts.iter().find(|(n, _)| n == name);
        let kept = || self.kept.iter().find(|k| &k.name == name);
        match read {
            Some((_, insert)) => Some(insert.document()),
            None => kept().map(|k| k.value.clone()),
        }
    }

    /// Takes component `name` away; whether it was there.
    pub(crate) fn remove(&mut self, name: &ComponentName) -> bool {
        let had = self.inserts.len() + self.kept.len();
        self.inserts.retain(|(n, _)| n != name);
        self.kept.retain(|k| &k.name != name);
        self.inserts.len() + self.kept.len() != had
    }

    /// Component `name` as `insert`, in place of what was there.
    pub(crate) fn put(&mut self, name: ComponentName, insert: Insert) {
        self.remove(&name);
        self.inserts.push((name, insert));
    }

    /// An unknown component as `kept`, in place of what was there.
    pub(crate) fn put_kept(&mut self, kept: KeptComponent) {
        self.remove(&kept.name);
        self.kept.push(kept);
    }
}
