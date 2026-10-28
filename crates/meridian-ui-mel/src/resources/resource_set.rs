//! [`ResourceSet`]: one `.mres` file.

use std::path::Path;

use substrate_data::Value;
use substrate_data::text::{self, Doc};

use super::read_entry::read_entry;
use super::{Culture, ResourceEntry, ResourceError, ResourceProblem};
use crate::snake_case;

/// The entries of one `<Set>.mres`, in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceSet {
    /// The set name: `Strings` for `Strings.mres` and `Strings.ru.mres`.
    pub name: String,
    /// The culture of a variant (`Strings.ru.mres`); `None` for the
    /// default set.
    pub culture: Option<Culture>,
    pub entries: Vec<ResourceEntry>,
}

impl ResourceSet {
    /// Parses and checks the text of `<name>.mres`; `path` is only used in
    /// errors.
    pub fn parse(path: &Path, name: &str, text: &str) -> Result<Self, ResourceError> {
        let at = |offset, problem| ResourceError::at(path, text, offset, problem);
        if !is_set_name(name) {
            return Err(at(0, ResourceProblem::InvalidSetName(name.to_string())));
        }
        let doc = text::parse(text)
            .map_err(|error| at(error.offset, ResourceProblem::Syntax(error.message)))?;
        let Value::Map(map) = &doc.value else {
            return Err(at(0, ResourceProblem::NotAnObject));
        };
        let mut set = Self {
            name: name.to_string(),
            culture: None,
            entries: Vec::new(),
        };
        for (index, (key, value)) in map.iter().enumerate() {
            let offset = entry_offset(&doc, index);
            let entry = read_entry(key, value).map_err(|problem| at(offset, problem))?;
            set.push(entry).map_err(|problem| at(offset, problem))?;
        }
        Ok(set)
    }

    /// Like [`Self::parse`], for the variant of `culture`.
    pub fn parse_variant(
        path: &Path,
        name: &str,
        culture: Culture,
        text: &str,
    ) -> Result<Self, ResourceError> {
        let mut set = Self::parse(path, name, text)?;
        set.culture = Some(culture);
        Ok(set)
    }

    /// The entry called `name`.
    pub fn entry(&self, name: &str) -> Option<&ResourceEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    /// Takes the value of every entry `variant` defines.
    pub(super) fn overlay(&mut self, variant: &Self) {
        for entry in &mut self.entries {
            if let Some(translated) = variant.entry(&entry.name) {
                entry.value = translated.value.clone();
            }
        }
    }

    fn push(&mut self, entry: ResourceEntry) -> Result<(), ResourceProblem> {
        if let Some(first) = self.entries.iter().find(|e| e.constant == entry.constant) {
            return Err(ResourceProblem::NameCollision {
                first: first.name.clone(),
                second: entry.name,
            });
        }
        self.entries.push(entry);
        Ok(())
    }
}

/// Where entry `index` starts in the text.
fn entry_offset(doc: &Doc, index: usize) -> usize {
    doc.spans
        .children
        .get(index)
        .map_or(0, |spans| spans.key.unwrap_or(spans.span).start as usize)
}

/// A set name is also a Rust type name.
fn is_set_name(name: &str) -> bool {
    name.starts_with(|ch: char| ch.is_ascii_alphabetic())
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// `screen.title` -> `SCREEN_TITLE`.
pub(super) fn constant_name(name: &str) -> String {
    name.split('.')
        .map(snake_case)
        .collect::<Vec<_>>()
        .join("_")
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests;
