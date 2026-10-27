//! [`MarkupExtension`]: the parsed inside of `{Name args...}`.

use super::bracket_depth::BracketDepth;
use super::markup_value::{MarkupValue, MarkupValueError, MarkupValueErrorKind, offset_in};
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
    /// Parses the text between the outer braces; error offsets are
    /// relative to `body`.
    pub(crate) fn parse_body(body: &str) -> Result<Self, MarkupValueError> {
        let trimmed = body.trim_start();
        let name_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        let name = QualifiedName::parse(&trimmed[..name_end]).map_err(|_| {
            MarkupValueError::at(MarkupValueErrorKind::InvalidName, offset_in(body, trimmed))
        })?;
        let mut extension = Self {
            name,
            positional: Vec::new(),
            named: Vec::new(),
        };
        let arguments = trimmed[name_end..].trim();
        if arguments.is_empty() {
            return Ok(extension);
        }
        let parts = BracketDepth::split_top_level(arguments)
            .map_err(|error| error.shifted(offset_in(body, arguments)))?;
        for part in parts {
            let argument = part.trim();
            extension
                .push_argument(argument)
                .map_err(|error| error.shifted(offset_in(body, argument)))?;
        }
        Ok(extension)
    }

    /// Adds one trimmed argument; error offsets are relative to it.
    fn push_argument(&mut self, argument: &str) -> Result<(), MarkupValueError> {
        let fail = |kind| Err(MarkupValueError::at(kind, 0));
        if argument.is_empty() {
            return fail(MarkupValueErrorKind::EmptyArgument);
        }
        let Some((key, value)) = split_named(argument) else {
            if !self.named.is_empty() {
                return fail(MarkupValueErrorKind::PositionalAfterNamed);
            }
            self.positional.push(MarkupValue::parse(argument)?);
            return Ok(());
        };
        if self.argument(key).is_some() {
            return fail(MarkupValueErrorKind::DuplicateArgument(key.to_string()));
        }
        let trimmed = value.trim();
        let at = offset_in(argument, trimmed);
        if trimmed.is_empty() {
            return Err(MarkupValueError::at(
                MarkupValueErrorKind::EmptyArgument,
                at,
            ));
        }
        let value = MarkupValue::parse(trimmed).map_err(|error| error.shifted(at))?;
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
