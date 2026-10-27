//! [`ExprError`]: why an `x:Bind` expression does not parse.

/// Why an `x:Bind` expression does not parse, and where: `offset` is a
/// byte offset into the expression text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("at byte {offset} of the expression: {kind}")]
pub struct ExprError {
    pub kind: ExprErrorKind,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExprErrorKind {
    #[error("unexpected end of expression")]
    UnexpectedEnd,
    #[error("unexpected '{0}'")]
    Unexpected(char),
    #[error("unterminated string literal")]
    UnterminatedText,
    #[error("expected a name")]
    ExpectedName,
    #[error("'{0}' is not a valid number")]
    InvalidNumber(String),
}
