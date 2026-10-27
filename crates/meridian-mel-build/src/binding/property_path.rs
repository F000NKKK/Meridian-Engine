//! [`PropertyPath`]: the `meridian-ui-core` constant a markup attribute
//! names (ADR 018 section 5).

use meridian_dsl_core::QualifiedName;
use meridian_ui_core::properties::by_name;

use crate::InvalidMarkup;

const PROPERTIES: &str = "::meridian_sdk::ui_core::properties";

/// `properties::<owner>::<NAME>`, resolved at build time against the
/// registry, so a typo in the markup is an error at its `.mel` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyPath {
    module: String,
    constant: String,
}

impl PropertyPath {
    /// Resolves attribute `attr` of an element `<tag ...>`: `Owner.Name`
    /// names the owner's property, a plain `name` is looked up on the tag
    /// and then among the properties every element has.
    pub fn resolve(tag: &str, attr: &QualifiedName) -> Result<Self, InvalidMarkup> {
        let member = pascal_case(attr.local());
        let candidates = match attr.owner() {
            Some(owner) => vec![format!("{owner}.{member}")],
            None => vec![format!("{tag}.{member}"), member],
        };
        candidates
            .iter()
            .find_map(|name| by_name(name))
            .map(|meta| Self::from_registered(meta.name))
            .ok_or_else(|| InvalidMarkup::UnknownProperty {
                element: tag.to_string(),
                property: attr.to_string(),
            })
    }

    /// `"Grid.Row"` -> `grid::ROW`; a bare `"Width"` -> `element::WIDTH`.
    fn from_registered(name: &str) -> Self {
        let (owner, member) = name.split_once('.').unwrap_or(("Element", name));
        Self {
            module: snake_case(owner),
            constant: snake_case(member).to_ascii_uppercase(),
        }
    }

    /// The constant as a Rust path.
    pub fn to_rust(&self) -> String {
        format!("{PROPERTIES}::{}::{}", self.module, self.constant)
    }
}

/// `font_size` and `fontSize` -> `FontSize`.
fn pascal_case(text: &str) -> String {
    text.split(['_', '-'])
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            })
        })
        .collect()
}

/// `TextBlock` -> `text_block`.
pub(crate) fn snake_case(text: &str) -> String {
    let mut snake = String::new();
    for (index, ch) in text.chars().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    snake
}

#[cfg(test)]
mod tests;
