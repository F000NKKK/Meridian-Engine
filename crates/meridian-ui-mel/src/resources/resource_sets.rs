//! [`ResourceSets`]: every `.mres` set of a build, by name.

use std::collections::BTreeMap;

use substrate_data::typed::{self, Spelled};

use super::{Culture, ResourceEntry, ResourceProblem, ResourceSet};
use crate::InvalidMarkup;

/// The kind of reference that names a resource: `@res:Set.name`.
const KIND: &str = "res";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResourceSets {
    sets: BTreeMap<String, ResourceSet>,
    /// Culture variants by set name and culture.
    variants: BTreeMap<(String, Culture), ResourceSet>,
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

    /// Adds `set` as the variant of `culture`. The default set must be in
    /// already; a variant may only define entries the default has, with
    /// the same type, and what it leaves out falls back to the default.
    pub fn insert_variant(
        &mut self,
        culture: Culture,
        mut set: ResourceSet,
    ) -> Result<(), ResourceProblem> {
        set.culture = Some(culture.clone());
        let default = self
            .sets
            .get(&set.name)
            .ok_or_else(|| ResourceProblem::VariantWithoutDefault(set.name.clone()))?;
        for entry in &set.entries {
            let Some(base) = default.entry(&entry.name) else {
                return Err(ResourceProblem::VariantUnknownEntry {
                    set: set.name.clone(),
                    entry: entry.name.clone(),
                });
            };
            if std::mem::discriminant(&base.value) != std::mem::discriminant(&entry.value) {
                let entry = entry.name.clone();
                return Err(ResourceProblem::VariantTypeMismatch { entry });
            }
        }
        let key = (set.name.clone(), culture.clone());
        if self.variants.contains_key(&key) {
            return Err(ResourceProblem::DuplicateSet(format!(
                "{}.{culture}",
                set.name
            )));
        }
        self.variants.insert(key, set);
        Ok(())
    }

    /// The sets as they read in `culture`: each entry takes the value of
    /// the most specific variant that defines it (`en-gb`, then `en`),
    /// else the default's.
    pub fn for_culture(&self, culture: &Culture) -> Self {
        let mut localized = Self::default();
        for default in self.sets.values() {
            let mut set = default.clone();
            for tag in culture.chain().into_iter().rev() {
                let Some(variant) = self.variants.get(&(default.name.clone(), tag)) else {
                    continue;
                };
                set.overlay(variant);
            }
            localized.sets.insert(set.name.clone(), set);
        }
        localized
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
