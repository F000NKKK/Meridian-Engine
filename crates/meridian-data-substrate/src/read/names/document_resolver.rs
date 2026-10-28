//! [`DocumentResolver`]: `@entity:` references resolved within the document.

use std::rc::Rc;

use substrate_data::{Kind, Path, Ref, RefKey, Resolve, Unresolved};

use super::document_names::DocumentNames;
use super::named::Named;
use super::pending_ref::PendingRef;
use crate::{EntityKey, EntityKind};

/// Resolves `@entity:` references against the document's own keys and
/// hands every other kind to the caller's resolver. In a prefab being
/// instanced it also renames them, as its entities are: a reference to
/// the prefab's root is to the instance, any other to `<instance>/<id>`.
/// A reference below an entity that instances a prefab is accepted and
/// remembered ([`PendingRef`]): expansion checks it.
pub(crate) struct DocumentResolver<'a> {
    names: Rc<DocumentNames>,
    outside: &'a mut dyn Resolve,
    pub(crate) pending: Vec<PendingRef>,
}

impl<'a> DocumentResolver<'a> {
    /// A resolver for the document `names` describes.
    pub(crate) fn new(names: Rc<DocumentNames>, outside: &'a mut dyn Resolve) -> Self {
        DocumentResolver {
            names,
            outside,
            pending: Vec::new(),
        }
    }
}

impl Resolve for DocumentResolver<'_> {
    fn reference(&mut self, r: &Ref, path: &Path) -> Result<RefKey, Unresolved> {
        if r.kind != EntityKind::kind_name() {
            return self.outside.reference(r, path);
        }
        let missing = || Unresolved::Missing(r.clone());
        let written = EntityKey::new(r.id.as_str()).map_err(|_| missing())?;
        let key = match self.names.named(&written).ok_or_else(missing)? {
            Named::Local(key) => key,
            Named::Generated { key, instance } => {
                let path = path.clone();
                let pending = PendingRef {
                    key: key.clone(),
                    instance,
                    path,
                };
                self.pending.push(pending);
                key
            }
        };
        RefKey::new(key.as_str()).map_err(|_| missing())
    }
}
