//! [`DslRegistry`]: tag names bound to [`crate::DslTag`] builders.

use std::any::Any;
use std::collections::HashMap;

use crate::{Attribute, BuiltNode, DslTag, Element, TagParseError};

type Builder = Box<dyn Fn(&[Attribute]) -> Result<Box<dyn Any>, TagParseError>>;

/// The set of tag names a DSL document may use, each bound to a
/// [`crate::DslTag`] impl via [`register`](Self::register). Built up once by
/// an application (registering `meridian-sdk`'s own built-in tags plus
/// any custom ones its game defines), then reused to
/// [`build`](Self::build) as many documents as it wants.
#[derive(Default)]
pub struct DslRegistry {
    builders: HashMap<String, Builder>,
}

impl DslRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `T::TAG_NAME` to `T::from_attrs`. Registering the same tag
    /// name twice replaces the previous binding — deliberately permissive
    /// (an application may want to override a built-in tag with its own
    /// definition) rather than an error.
    pub fn register<T: DslTag>(&mut self) {
        self.builders.insert(
            T::TAG_NAME.to_string(),
            Box::new(|attrs| T::from_attrs(attrs).map(|v| Box::new(v) as Box<dyn Any>)),
        );
    }

    /// Recursively builds `element` and every descendant into a
    /// [`BuiltNode`] tree. Fails closed: an element whose tag was never
    /// [`register`](Self::register)ed is a [`TagParseError`], not a
    /// silently-skipped node — a typo'd tag name in a DSL document must
    /// surface, not vanish.
    pub fn build(&self, element: &Element) -> Result<BuiltNode, TagParseError> {
        let tag = element.name.to_string();
        let builder = self.builders.get(&tag).ok_or_else(|| TagParseError {
            message: format!(
                "{}: unknown DSL tag '<{tag}>' — no type registered for it (typo, or missing DslRegistry::register call?)",
                element.span
            ),
        })?;
        let value = builder(&element.attrs).map_err(|e| TagParseError {
            message: format!("{}: <{tag}>: {}", element.span, e.message),
        })?;
        let children = element
            .children
            .iter()
            .map(|child| self.build(child))
            .collect::<Result<Vec<_>, _>>()?;
        let name = element
            .directive("Name")
            .and_then(|attr| attr.text())
            .map(str::to_string);
        Ok(BuiltNode {
            tag,
            name,
            value,
            children,
        })
    }
}

#[cfg(test)]
mod tests;
