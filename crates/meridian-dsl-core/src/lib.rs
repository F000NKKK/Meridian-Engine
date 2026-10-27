//! Generic tag-markup parser: `.mel` text in, an element/attribute tree
//! out. This crate is deliberately domain-blind — it has never heard of
//! `Mesh`, `RigidBody`, or any other scene concept. That knowledge lives
//! one layer up, in `meridian-sdk::dsl` (typed tag registration via
//! `#[dsl_tag]`) and further still in whatever custom tags a game
//! developer registers there. Splitting it this way is what makes the
//! DSL *extensible* rather than a fixed schema (ADR 015): the parser
//! never needs to change when a new tag is added, because it never knew
//! about tags in the first place, only about the syntax.
//!
//! Syntax (XAML-flavored, not a XAML implementation; ADR 017):
//! `<Tag attr="value"> ...children... </Tag>` or the self-closing
//! `<Tag attr="value" />`, one root element per document. On top of
//! that, as in XAML:
//!
//! - the `x:` directive prefix on attributes (`x:Class`, `x:Name`, ...);
//!   no other prefix exists, and elements never take one;
//! - dotted names: attached properties (`Grid.Row="1"`) and property
//!   elements (`<Entity.Components>` directly inside `<Entity>`, without
//!   attributes);
//! - markup-extension values (`{x:Bind count, Mode=TwoWay}`), parsed
//!   into [`MarkupValue::Extension`]; `{}` escapes a literal leading `{`.
//!
//! No CDATA, no processing instructions, no text nodes between elements.
//! Every element, attribute and error carries a [`Span`] (line/column)
//! for diagnostics and the editor.

mod parse;
mod syntax;
mod tag;

pub use parse::{ParseError, ParseErrorKind, Span, parse};
pub use syntax::{
    Attribute, Element, MarkupExtension, MarkupValue, MarkupValueError, Prefix, QualifiedName,
    QualifiedNameError,
};
pub use tag::{BuiltNode, DslRegistry, DslTag, TagParseError, text_attr};
