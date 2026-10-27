//! [`MarkupExtension`]: the parsed inside of `{Name args...}`.

use super::bracket_depth::BracketDepth;
use super::markup_value::{MarkupValue, MarkupValueError};
use super::qualified_name::{QualifiedName, is_identifier};

/// `{Name positional, ..., Key=value, ...}`. Positional arguments come
/// first, as in XAML; argument text is trimmed.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkupExtension {
    pub name: QualifiedName,
    pub positional: Vec<MarkupValue>,
    pub named: Vec<(QualifiedName, MarkupValue)>,
}

impl MarkupExtension {
    /// Parses the text between the outer braces.
    pub(crate) fn parse_body(body: &str) -> Result<Self, MarkupValueError> {
        let body = body.trim_start();
        let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
        let name =
            QualifiedName::parse(&body[..name_end]).map_err(|_| MarkupValueError::InvalidName)?;
        let mut extension = Self {
            name,
            positional: Vec::new(),
            named: Vec::new(),
        };
        let arguments = body[name_end..].trim();
        if !arguments.is_empty() {
            for argument in BracketDepth::split_top_level(arguments)? {
                extension.push_argument(argument.trim())?;
            }
        }
        Ok(extension)
    }

    fn push_argument(&mut self, argument: &str) -> Result<(), MarkupValueError> {
        if argument.is_empty() {
            return Err(MarkupValueError::EmptyArgument);
        }
        let Some((key, value)) = split_named(argument) else {
            if !self.named.is_empty() {
                return Err(MarkupValueError::PositionalAfterNamed);
            }
            self.positional.push(MarkupValue::parse(argument)?);
            return Ok(());
        };
        if self.argument(key).is_some() {
            return Err(MarkupValueError::DuplicateArgument(key.to_string()));
        }
        let value = value.trim();
        if value.is_empty() {
            return Err(MarkupValueError::EmptyArgument);
        }
        let value = MarkupValue::parse(value)?;
        self.named.push((QualifiedName::plain(key), value));
        Ok(())
    }

    /// The named argument `key`, if given.
    pub fn argument(&self, key: &str) -> Option<&MarkupValue> {
        self.named
            .iter()
            .find(|(name, _)| name.is_plain(key))
            .map(|(_, value)| value)
    }
}

/// `Key=value` with `Key` an identifier and a single `=` (so the
/// expression `a == b` stays positional).
fn split_named(argument: &str) -> Option<(&str, &str)> {
    let eq = argument.find('=')?;
    let key = argument[..eq].trim_end();
    let value = &argument[eq + 1..];
    (is_identifier(key) && !value.starts_with('=')).then_some((key, value))
}
