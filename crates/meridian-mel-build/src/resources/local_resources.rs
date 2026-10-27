//! [`LocalResources`]: the keyed values of one `<Owner.Resources>` block.

use std::collections::BTreeMap;

use meridian_dsl_core::{Attribute, Element};

use crate::ui_node::UiNodeError;
use crate::{InvalidMarkup, ResourceTag, ResourceValue, read_markup_value};

/// `<Owner.Resources>` holds `<String|Number|Bool|Color x:Key=".."
/// Value=".." />`; they are visible to `{StaticResource Key}` on the
/// owner and everything below it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocalResources {
    entries: BTreeMap<String, ResourceValue>,
}

impl LocalResources {
    /// Reads the block `element` (the `<Owner.Resources>` element).
    pub fn from_element(element: &Element) -> Result<Self, UiNodeError> {
        let mut resources = Self::default();
        for child in &element.children {
            let (key, value) = read_resource(child)?;
            if resources.entries.insert(key.clone(), value).is_some() {
                return Err((child.span, InvalidMarkup::DuplicateResourceKey(key)));
            }
        }
        Ok(resources)
    }

    /// The value of `key`, if this block defines it.
    pub fn get(&self, key: &str) -> Option<&ResourceValue> {
        self.entries.get(key)
    }
}

fn read_resource(element: &Element) -> Result<(String, ResourceValue), UiNodeError> {
    let tag = element.name.to_string();
    let at = |reason| (element.span, reason);
    let Some(kind) = ResourceTag::parse(&tag).filter(|_| element.children.is_empty()) else {
        return Err(at(InvalidMarkup::InvalidResourceElement(tag)));
    };
    let key = element
        .directive("Key")
        .and_then(Attribute::text)
        .ok_or_else(|| at(InvalidMarkup::MissingResourceKey(tag.clone())))?;
    if !is_key(key) {
        return Err(at(InvalidMarkup::InvalidResourceKey(key.to_string())));
    }
    let text = element
        .attrs
        .iter()
        .find(|attr| attr.name.is_plain("Value"))
        .and_then(Attribute::text)
        .ok_or_else(|| at(InvalidMarkup::MissingResourceValue(key.to_string())))?;
    let value = read_markup_value(key, kind, text)
        .map_err(|problem| at(InvalidMarkup::InvalidResourceValue(problem)))?;
    Ok((key.to_string(), value))
}

/// A plain identifier: a dot would make it a `.mres` entry reference.
fn is_key(key: &str) -> bool {
    key.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
