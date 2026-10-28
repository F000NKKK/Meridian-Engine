//! [`DocumentResolver`]: `@entity:` references resolved within the document.

use std::collections::HashSet;

use substrate_data::{Kind, Path, Ref, RefKey, Resolve, Unresolved, ValueRef};

use super::DocumentRole;
use crate::document_keys::{ENTITIES, ID};
use crate::{EntityKey, EntityKind};

/// Resolves `@entity:` references against the document's own keys and
/// hands every other kind to the caller's resolver. In a prefab being
/// instanced it also renames them, as its entities are: a reference to
/// the prefab's root is to the instance, any other to `<instance>/<id>`.
pub(crate) struct DocumentResolver<'a> {
    keys: HashSet<EntityKey>,
    root: Option<EntityKey>,
    role: &'a DocumentRole,
    outside: &'a mut dyn Resolve,
}

impl<'a> DocumentResolver<'a> {
    /// The keys of `root`'s entities, read loosely before anything is
    /// checked, so a reference may name an entity further down.
    pub(crate) fn new(
        root: &ValueRef<'_>,
        outside: &'a mut dyn Resolve,
        role: &'a DocumentRole,
    ) -> Self {
        let mut keys = Vec::new();
        let listed = root.get(ENTITIES).and_then(|e| e.as_list());
        for e in listed.into_iter().flatten() {
            keys.push(
                e.get(ID)
                    .and_then(|id| id.as_str())
                    .and_then(|k| EntityKey::new(k).ok()),
            );
        }
        DocumentResolver {
            root: keys.first().cloned().flatten(),
            keys: keys.into_iter().flatten().collect(),
            role,
            outside,
        }
    }
}

impl Resolve for DocumentResolver<'_> {
    fn reference(&mut self, r: &Ref, path: &Path) -> Result<RefKey, Unresolved> {
        if r.kind != EntityKind::kind_name() {
            return self.outside.reference(r, path);
        }
        let local = EntityKey::new(r.id.as_str())
            .ok()
            .filter(|k| self.keys.contains(k));
        let Some(local) = local else {
            return Err(Unresolved::Missing(r.clone()));
        };
        let is_root = self.root.as_ref() == Some(&local);
        let renamed = self.role.key_of(local, is_root);
        RefKey::new(renamed.as_str()).map_err(|_| Unresolved::Missing(r.clone()))
    }
}
