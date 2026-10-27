//! Generic tag-markup parser: text in, a tag/attribute/children tree
//! out. This crate is deliberately domain-blind — it has never heard of
//! `Mesh`, `RigidBody`, or any other scene concept. That knowledge lives
//! one layer up, in `meridian-sdk::dsl` (typed tag registration via
//! `#[dsl_tag]`) and further still in whatever custom tags a game
//! developer registers there. Splitting it this way is what makes the
//! DSL *extensible* rather than a fixed schema (see
//! `docs/adr/*dsl*.md` for the decision this crate implements): the
//! parser never needs to change when a new tag is added, because it
//! never knew about tags in the first place, only about `<Name attr="value">`
//! syntax.
//!
//! Layer: shared kernel; bounded context: scene-markup (`craftbook.toml`,
//! docs/context-map.md).
//!
//! Syntax, deliberately small (XAML-flavored, not a XAML implementation):
//! `<Tag attr="value" other="123"> ...children... </Tag>` or the
//! self-closing `<Tag attr="value" />`. One root element per document.
//! No namespaces, no CDATA, no processing instructions, no text nodes
//! between elements (a DSL document describes structure, not prose) —
//! anything beyond that is deliberately out of scope until a real
//! caller needs it.

#![forbid(unsafe_code)]

mod element;
mod parse;
mod parse_error;
mod parser;
mod tag;
#[cfg(test)]
mod tests;

pub use element::Element;
pub use parse::parse;
pub use parse_error::ParseError;
use parser::Parser;
pub use tag::{BuiltNode, DslRegistry, DslTag, TagParseError};
