//! Attribute values: plain text, or a XAML markup extension in braces
//! (`{x:Bind count, Mode=TwoWay}`, `{StaticResource Accent}`). Parsed once
//! here, so later layers never re-scan a value for braces. A value that
//! must start with a literal `{` is written with the XAML escape `{}`
//! (`"{}{not an extension}"` is the text `{not an extension}`).
//!
//! The extension's meaning is not interpreted here: an `x:Bind`
//! expression stays the positional text `count`; the `.mel` generator
//! compiles it (ADR 017).

use std::fmt;

use super::bracket_depth::BracketDepth;
use super::markup_extension::MarkupExtension;

/// One attribute value or extension argument.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkupValue {
    Text(String),
    Extension(MarkupExtension),
}

/// Why a value starting with `{` is not a well-formed extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkupValueError {
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
            return Err(MarkupValueError::TrailingContent);
        }
        MarkupExtension::parse_body(&value[1..end]).map(Self::Extension)
    }

    /// The text, when this is not an extension.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Extension(_) => None,
        }
    }
}

impl fmt::Display for MarkupValueError {
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
