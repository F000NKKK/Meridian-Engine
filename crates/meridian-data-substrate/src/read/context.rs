//! [`ReadContext`]: what reading a document needs from the caller.

use substrate_data::{Resolve, ValueRef};

use super::DocumentRole;
use super::document_reader::DocumentReader;
use super::reading::Reading;
use crate::draft::DocumentDraft;
use crate::prefab::PrefabSource;
use crate::{ComponentRegistry, PrefabPath};

/// What reading a document needs: the registry, the caller's resolver for
/// references that are not entities, and, when prefabs are instanced, their
/// source and the chain of prefabs being read (to find cycles).
pub(crate) struct ReadContext<'a> {
    pub(crate) registry: &'a ComponentRegistry,
    pub(crate) resolver: &'a mut dyn Resolve,
    pub(crate) prefabs: Option<&'a mut dyn PrefabSource>,
    pub(crate) chain: Vec<PrefabPath>,
}

impl<'a> ReadContext<'a> {
    /// A context that leaves `prefab` fields alone (only checked by `resolver`).
    pub(crate) fn new(registry: &'a ComponentRegistry, resolver: &'a mut dyn Resolve) -> Self {
        ReadContext {
            registry,
            resolver,
            prefabs: None,
            chain: Vec::new(),
        }
    }

    /// The same, instancing every `prefab` field from `prefabs`.
    pub(crate) fn instancing(mut self, prefabs: &'a mut dyn PrefabSource) -> Self {
        self.prefabs = Some(prefabs);
        self
    }

    /// Reads the document `root`, then instances its prefabs when this
    /// context does. The draft is there only if nothing is wrong.
    pub(crate) fn read(&mut self, role: &DocumentRole, root: &ValueRef<'_>) -> Reading {
        let mut reading = DocumentReader::read((self.registry, role), root, self.resolver);
        if self.prefabs.is_none() {
            return reading;
        }
        if let Some(DocumentDraft { name, entities }) = reading.draft.take() {
            match self.expand(entities) {
                Ok(entities) => reading.draft = Some(DocumentDraft { name, entities }),
                Err(found) => reading.findings.extend(found),
            }
        }
        reading
    }
}
