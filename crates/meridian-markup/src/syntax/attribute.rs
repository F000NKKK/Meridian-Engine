//! [`Attribute`]: one `name="value"` pair of an element.

use super::markup_value::MarkupValue;
use super::qualified_name::QualifiedName;
use crate::Span;

/// One `name="value"` pair. `name` may be a directive (`x:Name`) or an
/// attached property (`Grid.Row`).
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: QualifiedName,
    pub value: MarkupValue,
    /// Where the attribute's name starts.
    pub span: Span,
}

impl Attribute {
    /// The value's text; `None` for a markup extension.
    pub fn text(&self) -> Option<&str> {
        self.value.as_text()
    }
}
