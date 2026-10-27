//! [`DocumentResolver`]: `@entity:` references resolved within the document.

use std::collections::HashSet;

use substrate_data::{Kind, Path, Ref, RefKey, Resolve, Unresolved, ValueRef};

use crate::document_keys::{ENTITIES, ID};
use crate::{EntityKey, EntityKind};

/// Resolves `@entity:` references against the document's own keys and
/// hands every other kind to the caller's resolver.
pub(crate) struct DocumentResolver<'a> {
    keys: HashSet<EntityKey>,
    outside: &'a mut dyn Resolve,
}

impl<'a> DocumentResolver<'a> {
    /// The keys of `root`'s entities, read loosely before anything is
    /// checked, so a reference may name an entity further down.
    pub(crate) fn new(root: &ValueRef<'_>, outside: &'a mut dyn Resolve) -> Self {
        let keys = root
            .get(ENTITIES)
            .and_then(|e| e.as_list())
            .into_iter()
            .flatten()
            .filter_map(|e| EntityKey::new(e.get(ID)?.as_str()?).ok())
            .collect();
        DocumentResolver { keys, outside }
    }
}

impl Resolve for DocumentResolver<'_> {
    fn reference(&mut self, r: &Ref, path: &Path) -> Result<RefKey, Unresolved> {
        if r.kind != EntityKind::kind_name() {
            return self.outside.reference(r, path);
        }
        let known = EntityKey::new(r.id.as_str()).is_ok_and(|k| self.keys.contains(&k));
        if known {
            Ok(r.id.clone())
        } else {
            Err(Unresolved::Missing(r.clone()))
        }
    }
}
