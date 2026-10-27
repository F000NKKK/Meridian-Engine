//! [`UiLiteral`]: a plain attribute of a UI element, set when the tree is
//! built.

use meridian_markup::Span;

use crate::PropertyPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLiteral {
    /// The property it sets.
    pub path: PropertyPath,
    /// The attribute as written (`font_size`, `Grid.Row`).
    pub property: String,
    /// The attribute text, parsed by `meridian-ui-domain` at run time.
    pub text: String,
    pub span: Span,
}
