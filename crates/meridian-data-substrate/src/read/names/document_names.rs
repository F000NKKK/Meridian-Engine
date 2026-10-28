//! [`DocumentNames`]: the entity keys a document is read with.

use std::collections::HashSet;

use substrate_data::ValueRef;

use super::named::Named;
use crate::EntityKey;
use crate::document_keys::{ENTITIES, ID, PREFAB};
use crate::read::DocumentRole;

/// What a document's `@entity:` names mean: its own keys (renamed when it
/// is a prefab being instanced), and keys below an entity that instances a
/// prefab, which that prefab makes and only expansion can check.
#[derive(Debug)]
pub(crate) struct DocumentNames {
    keys: HashSet<EntityKey>,
    instances: HashSet<EntityKey>,
    root: Option<EntityKey>,
    role: DocumentRole,
}

impl DocumentNames {
    /// The keys of `root`'s entities, read loosely before anything is
    /// checked, so a name may refer to an entity further down.
    pub(crate) fn new(root: &ValueRef<'_>, role: &DocumentRole) -> Self {
        let (mut keys, mut instances, mut first) = (HashSet::new(), HashSet::new(), None);
        let listed = root.get(ENTITIES).and_then(|e| e.as_list());
        for (i, e) in listed.into_iter().flatten().enumerate() {
            let key = e.get(ID).and_then(|id| id.as_str());
            let Some(key) = key.and_then(|k| EntityKey::new(k).ok()) else {
                continue;
            };
            if i == 0 {
                first = Some(key.clone());
            }
            if e.get(PREFAB).is_some() {
                instances.insert(key.clone());
            }
            keys.insert(key);
        }
        DocumentNames {
            keys,
            instances,
            root: first,
            role: role.clone(),
        }
    }

    /// The key a document entity written as `local` has once read.
    pub(crate) fn keyed(&self, local: &EntityKey) -> EntityKey {
        self.role
            .key_of(local.clone(), self.root.as_ref() == Some(local))
    }

    /// What `written` names, if anything this document can have.
    pub(crate) fn named(&self, written: &EntityKey) -> Option<Named> {
        if self.keys.contains(written) {
            return Some(Named::Local(self.keyed(written)));
        }
        let (instance, below) = self.instance_of(written)?;
        let instance = self.keyed(instance);
        let key = EntityKey::new(&format!("{instance}{below}")).ok()?;
        Some(Named::Generated { key, instance })
    }

    /// The longest instance of this document whose key `written` is below,
    /// and the rest of `written` (starting with `/`).
    fn instance_of<'n>(&'n self, written: &'n EntityKey) -> Option<(&'n EntityKey, &'n str)> {
        let below = |i: &&EntityKey| {
            let rest = written.as_str().strip_prefix(i.as_str())?;
            rest.starts_with('/').then_some(rest)
        };
        let found = self.instances.iter().filter(|i| below(i).is_some());
        let longest = found.max_by_key(|i| i.as_str().len())?;
        Some((longest, below(&longest)?))
    }
}
