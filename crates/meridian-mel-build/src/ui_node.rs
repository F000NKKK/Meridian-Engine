//! [`UiNode`]: an element of a `.mel` document that builds a
//! `meridian-ui-core` element.

use meridian_dsl_core::{Attribute, Element, MarkupValue, Span};
use meridian_ui_core::ElementKind;

use crate::{InvalidMarkup, NamedElement, PropertyPath, UiLiteral};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiNode {
    pub kind: ElementKind,
    /// The `Names` field when the element has an `x:Name`.
    pub field: Option<String>,
    /// Plain attributes, in document order; bound ones are applied by
    /// `apply_bindings` instead.
    pub literals: Vec<UiLiteral>,
    pub children: Vec<UiNode>,
}

/// A rule broken at `span`.
pub type UiNodeError = (Span, InvalidMarkup);

impl UiNode {
    /// Whether `element` is the root of a UI document: its tag is an
    /// element kind of `meridian-ui-core`.
    pub fn is_ui_root(element: &Element) -> bool {
        element_kind(element).is_some()
    }

    /// Checks `element` and its subtree; `names` are the document's
    /// `x:Name`s.
    pub fn from_element(element: &Element, names: &[NamedElement]) -> Result<Self, UiNodeError> {
        let tag = element.name.to_string();
        let Some(kind) = element_kind(element) else {
            return Err((element.span, InvalidMarkup::UnknownElement(tag)));
        };
        let field = element
            .directive("Name")
            .and_then(Attribute::text)
            .and_then(|name| names.iter().find(|named| named.name == name))
            .map(|named| named.field.clone());
        let mut literals = Vec::new();
        for attr in element
            .attrs
            .iter()
            .filter(|attr| attr.name.prefix().is_none())
        {
            if let Some(literal) = literal(&tag, attr)? {
                literals.push(literal);
            }
        }
        let children = element
            .children
            .iter()
            .map(|child| Self::from_element(child, names))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            kind,
            field,
            literals,
            children,
        })
    }
}

fn element_kind(element: &Element) -> Option<ElementKind> {
    if element.is_property_element() {
        return None;
    }
    ElementKind::from_tag(&element.name.to_string())
}

/// `None` for an `{x:Bind}`, which `apply_bindings` writes.
fn literal(tag: &str, attr: &Attribute) -> Result<Option<UiLiteral>, UiNodeError> {
    let text = match &attr.value {
        MarkupValue::Text(text) => text,
        MarkupValue::Extension(extension) if extension.name.is_directive("Bind") => {
            return Ok(None);
        }
        MarkupValue::Extension(extension) => {
            let reason = InvalidMarkup::UnsupportedExtension(extension.name.to_string());
            return Err((attr.span, reason));
        }
    };
    let path = PropertyPath::resolve(tag, &attr.name).map_err(|reason| (attr.span, reason))?;
    Ok(Some(UiLiteral {
        path,
        property: attr.name.to_string(),
        text: text.clone(),
        span: attr.span,
    }))
}

#[cfg(test)]
mod tests;
