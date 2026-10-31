//! [`Element`]: one node of the parsed tree, with its attributes and
//! children.

use super::attribute::Attribute;
use super::qualified_name::QualifiedName;
use crate::Span;

/// One parsed element: its name, its attributes (in source order,
/// duplicates rejected at parse time rather than silently
/// last-write-wins — a duplicate attribute is almost always a typo, not
/// intent), and its children in source order.
///
/// A child whose name has an owner (`<Entity.Components>` inside
/// `<Entity>`) is a property element, as in XAML: the parser checks that
/// its owner is the enclosing element and that it has no attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// Parsed element name.
    pub name: QualifiedName,
    /// Attributes in source order.
    pub attrs: Vec<Attribute>,
    /// Child elements in source order.
    pub children: Vec<Element>,
    /// Where the element's `<` is.
    pub span: Span,
}

impl Element {
    /// The first plain (unprefixed, ownerless) attribute named `name` —
    /// attributes are stored as a `Vec` (not a `HashMap`) so source order
    /// survives for tools that echo it back (formatters, the designer),
    /// but lookups still want to feel like map access.
    pub fn attr(&self, name: &str) -> Option<&Attribute> {
        self.attrs.iter().find(|attr| attr.name.is_plain(name))
    }

    /// The directive attribute `x:local` (`x:Name`, `x:Class`, ...).
    pub fn directive(&self, local: &str) -> Option<&Attribute> {
        self.attrs.iter().find(|attr| attr.name.is_directive(local))
    }

    /// Whether this is a property element (`Owner.Property`).
    pub fn is_property_element(&self) -> bool {
        self.name.owner().is_some()
    }
}
