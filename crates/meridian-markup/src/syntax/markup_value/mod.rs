//! Attribute values: plain text, or a XAML markup extension in braces
//! (`{x:Bind count, Mode=TwoWay}`, `{StaticResource Accent}`). Parsed once
//! here, so later layers never re-scan a value for braces. A value that
//! must start with a literal `{` is written with the XAML escape `{}`
//! (`"{}{not an extension}"` is the text `{not an extension}`).
//!
//! The extension's meaning is not interpreted here: an `x:Bind`
//! expression stays the positional text `count`; the `.meui` generator
//! compiles it (ADR 017).

use std::fmt;

use super::bracket_depth::BracketDepth;
use super::markup_extension::MarkupExtension;

/// One attribute value or extension argument.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkupValue {
    /// Literal text without markup-extension semantics.
    Text(String),
    /// A parsed extension inside braces.
    Extension(MarkupExtension),
}

/// Why a value starting with `{` is not a well-formed extension, and
/// where: `offset` is a byte offset into the value [`MarkupValue::parse`]
/// was given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkupValueError {
    /// The markup-extension rule that was violated.
    pub kind: MarkupValueErrorKind,
    /// Zero-based byte offset of the problem within the value.
    pub offset: usize,
}

/// Why a markup-extension value is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkupValueErrorKind {
    /// No closing `}`, or a `(`/`[`/`{`/`'` left open inside it.
    Unbalanced,
    /// Text after the closing `}`.
    TrailingContent,
    /// Braces with no extension name, or a malformed one.
    InvalidName,
    /// An empty argument between commas.
    EmptyArgument,
    /// A positional argument after a named one.
    PositionalAfterNamed,
    /// The same named argument twice.
    DuplicateArgument(String),
}

impl MarkupValue {
    /// Parses a complete, already unescaped attribute value.
    pub fn parse(value: &str) -> Result<Self, MarkupValueError> {
        if let Some(text) = value.strip_prefix("{}") {
            return Ok(Self::Text(text.to_string()));
        }
        if !value.starts_with('{') {
            return Ok(Self::Text(value.to_string()));
        }
        let end = BracketDepth::closing_brace(value)?;
        if end + 1 != value.len() {
            return Err(MarkupValueError::at(
                MarkupValueErrorKind::TrailingContent,
                end + 1,
            ));
        }
        MarkupExtension::parse_body(&value[1..end])
            .map(Self::Extension)
            .map_err(|error| error.shifted(1))
    }

    /// The text, when this is not an extension.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Extension(_) => None,
        }
    }
}

impl MarkupValueError {
    pub(crate) fn at(kind: MarkupValueErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    /// The same error, for a value that sits `by` bytes into another.
    pub(crate) fn shifted(self, by: usize) -> Self {
        Self {
            offset: self.offset + by,
            ..self
        }
    }
}

/// Byte offset of `part` inside `whole`; `part` must be a subslice.
pub(crate) fn offset_in(whole: &str, part: &str) -> usize {
    part.as_ptr() as usize - whole.as_ptr() as usize
}

impl fmt::Display for MarkupValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at byte {} of the value: {}", self.offset, self.kind)
    }
}

impl fmt::Display for MarkupValueErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unbalanced => f.write_str("unbalanced brackets or quotes in markup extension"),
            Self::TrailingContent => f.write_str("text after the markup extension's closing '}'"),
            Self::InvalidName => f.write_str("missing or malformed markup extension name"),
            Self::EmptyArgument => f.write_str("empty markup extension argument"),
            Self::PositionalAfterNamed => f.write_str("positional argument after a named one"),
            Self::DuplicateArgument(key) => write!(f, "duplicate argument '{key}'"),
        }
    }
}

impl std::error::Error for MarkupValueError {}

#[cfg(test)]
mod tests;
