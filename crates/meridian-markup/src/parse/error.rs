//! [`ParseError`]: what went wrong while parsing a `.meui` document, and
//! where.

use std::fmt;

use super::span::Span;
use crate::MarkupValueErrorKind;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    Expected(char),
    ExpectedIdentifier,
    UnterminatedString,
    InvalidEscape,
    InvalidUtf8,
    /// A name with a prefix other than `x:`.
    UnknownPrefix(String),
    /// An element name with the `x:` prefix; directives are attributes.
    DirectiveElement(String),
    DuplicateAttribute(String),
    MismatchedClosingTag {
        expected: String,
        found: String,
    },
    UnterminatedElement(String),
    /// Content after the root element.
    TrailingContent,
    /// A property element at the root of the document.
    PropertyElementAtRoot(String),
    /// `<Owner.Property>` inside an element that is not `Owner`.
    PropertyElementOwner {
        property: String,
        parent: String,
    },
    /// A property element with attributes.
    PropertyElementAttributes(String),
    /// An attribute value that starts like a markup extension but is
    /// not one.
    Extension {
        attribute: String,
        error: MarkupValueErrorKind,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.span, self.kind)
    }
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(ch) => write!(f, "expected '{ch}'"),
            Self::ExpectedIdentifier => f.write_str("expected an identifier"),
            Self::UnterminatedString => f.write_str("unterminated string literal"),
            Self::InvalidEscape => f.write_str("invalid escape in string literal"),
            Self::InvalidUtf8 => f.write_str("invalid UTF-8"),
            Self::UnknownPrefix(prefix) => {
                write!(f, "unknown prefix '{prefix}:' (only 'x:' is defined)")
            }
            Self::DirectiveElement(name) => {
                write!(
                    f,
                    "'<{name}>': 'x:' directives are attributes, not elements"
                )
            }
            Self::DuplicateAttribute(name) => write!(f, "duplicate attribute '{name}'"),
            Self::MismatchedClosingTag { expected, found } => write!(
                f,
                "mismatched closing tag: expected '</{expected}>', found '</{found}>'"
            ),
            Self::UnterminatedElement(name) => write!(f, "unterminated element '<{name}>'"),
            Self::TrailingContent => f.write_str("expected end of document after root element"),
            Self::PropertyElementAtRoot(name) => {
                write!(f, "property element '<{name}>' cannot be the document root")
            }
            Self::PropertyElementOwner { property, parent } => {
                write!(f, "property element '<{property}>' inside '<{parent}>'")
            }
            Self::PropertyElementAttributes(name) => {
                write!(f, "property element '<{name}>' cannot have attributes")
            }
            Self::Extension { attribute, error } => {
                write!(f, "attribute '{attribute}': {error}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl meridian_diagnostics::EngineError for ParseError {}
