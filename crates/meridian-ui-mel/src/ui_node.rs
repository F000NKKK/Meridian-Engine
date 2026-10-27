//! [`UiNode`]: an element of a `.mel` document that builds a
//! `meridian-ui-domain` element.

use meridian_markup::{Element, Span};
use meridian_ui_domain::ElementKind;

use crate::ui_builder::UiBuilder;
use crate::{InvalidMarkup, NamedElement, ResourceSets, UiLiteral};

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
    /// element kind of `meridian-ui-domain`.
    pub fn is_ui_root(element: &Element) -> bool {
        element_kind(element).is_some()
    }

    /// Checks `element` and its subtree; `names` are the document's
    /// `x:Name`s and `resources` resolve `@res:` and dotted
    /// `{StaticResource}` references.
    pub fn from_element(
        element: &Element,
        names: &[NamedElement],
        resources: &ResourceSets,
    ) -> Result<Self, UiNodeError> {
        UiBuilder::new(names, resources).node(element)
    }
}

pub(crate) fn element_kind(element: &Element) -> Option<ElementKind> {
    if element.is_property_element() {
        return None;
    }
    ElementKind::from_tag(&element.name.to_string())
}

#[cfg(test)]
mod tests;
