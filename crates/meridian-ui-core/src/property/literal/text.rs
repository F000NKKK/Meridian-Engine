//! String literals under the Substrate-Data `@` rule.

use super::LiteralError;

/// As spelled, except that a leading `@@` is a literal `@` and any other
/// leading `@` is a reference for the caller to resolve.
pub(in crate::property) fn text(text: &str) -> Result<String, LiteralError> {
    match text.strip_prefix('@') {
        None => Ok(text.to_string()),
        Some(rest) if rest.starts_with('@') => Ok(rest.to_string()),
        Some(_) => Err(LiteralError::Reference(text.to_string())),
    }
}
