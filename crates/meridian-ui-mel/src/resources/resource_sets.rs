//! [`ResourceSets`]: every `.mres` set of a build, by name.

use std::collections::BTreeMap;

use substrate_data::typed::{self, Spelled};

use super::{ResourceEntry, ResourceSet};
use crate::InvalidMarkup;

/// The kind of reference that names a resource: `@res:Set.name`.
const KIND: &str = "res";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResourceSets {
    sets: BTreeMap<String, ResourceSet>,
}

impl ResourceSets {
    /// Adds `set`; `false` when a set of that name is already there.
    pub fn insert(&mut self, set: ResourceSet) -> bool {
        if self.sets.contains_key(&set.name) {
            return false;
        }
        self.sets.insert(set.name.clone(), set);
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = &ResourceSet> {
        self.sets.values()
    }

    /// The entry `Set.name`.
    pub fn entry(&self, reference: &str) -> Option<&ResourceEntry> {
        let (set, name) = reference.split_once('.')?;
        self.sets.get(set)?.entry(name)
    }

    /// The text of attribute `text` once `@res:Set.name` is replaced by
    /// the resource's value. Plain text, `@@` escapes and references of
    /// other kinds (resolved by the program) pass through unchanged.
    pub fn resolve(&self, text: &str) -> Result<String, InvalidMarkup> {
        if !text.starts_with('@') {
            return Ok(text.to_string());
        }
        match typed::classify(text) {
            Ok(Spelled::Ref(reference)) if reference.kind.as_str() == KIND => {
                let id = reference.id.as_str();
                self.entry(id)
                    .map(|entry| entry.value.literal())
                    .ok_or_else(|| InvalidMarkup::UnknownResource(id.to_string()))
            }
            Ok(Spelled::Bytes(_)) => Err(InvalidMarkup::InvalidLiteral(
                "bytes cannot be an attribute value".to_string(),
            )),
            Ok(_) => Ok(text.to_string()),
            Err(error) => Err(InvalidMarkup::InvalidLiteral(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests;
