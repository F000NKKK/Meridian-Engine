//! [`ParseError`]: what went wrong while parsing a `.meui` document, and
//! where.

use std::fmt;

use super::span::Span;
use crate::MarkupValueErrorKind;

/// A markup parsing failure and its source position.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// The syntax rule that was violated.
    pub kind: ParseErrorKind,
    /// Where the invalid input starts.
    pub span: Span,
}

/// The syntax rule violated by a markup document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// The parser expected the contained character.
    Expected(char),
    /// The parser expected an identifier.
    ExpectedIdentifier,
    /// A string literal reaches the end of input without a closing quote.
    UnterminatedString,
    /// A string literal contains an unsupported escape sequence.
    InvalidEscape,
    /// The input is not valid UTF-8.
    InvalidUtf8,
    /// A name with a prefix other than `x:`.
    UnknownPrefix(String),
    /// An element name with the `x:` prefix; directives are attributes.
    DirectiveElement(String),
    /// An attribute occurs more than once on one element.
    DuplicateAttribute(String),
    /// A closing tag does not match the open element.
    MismatchedClosingTag {
        /// Name of the element that must be closed.
        expected: String,
        /// Name found in the closing tag.
        found: String,
    },
    /// An element is not closed before the end of input.
    UnterminatedElement(String),
    /// Content after the root element.
    TrailingContent,
    /// A property element at the root of the document.
    PropertyElementAtRoot(String),
    /// `<Owner.Property>` inside an element that is not `Owner`.
    PropertyElementOwner {
        /// Full property element name.
        property: String,
        /// Name of the enclosing element.
        parent: String,
    },
    /// A property element with attributes.
    PropertyElementAttributes(String),
    /// An attribute value that starts like a markup extension but is not one.
    Extension {
        /// Attribute containing the malformed value.
        attribute: String,
        /// Why the markup extension is invalid.
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
