//! Why a markup literal was rejected.

/// A markup literal that does not parse as the property's type.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LiteralError {
    /// The text is not a valid value; `expected` says what would be.
    #[error("expected {expected}, found `{found}`")]
    Invalid {
        expected: &'static str,
        found: String,
    },
    /// A Substrate-Data reference (`@kind:id`). Resolving references needs
    /// a resource lookup this crate does not have; the caller handles it.
    #[error("reference `{0}` must be resolved by the caller")]
    Reference(String),
}

impl LiteralError {
    pub(super) fn invalid(expected: &'static str, found: &str) -> Self {
        Self::Invalid {
            expected,
            found: found.to_string(),
        }
    }
}

impl meridian_diagnostics::EngineError for LiteralError {}
