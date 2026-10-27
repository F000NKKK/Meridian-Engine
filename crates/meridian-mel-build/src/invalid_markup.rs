//! [`InvalidMarkup`]: a rule of ADR 017 that a parsed `.mel` document breaks.

/// A rule of ADR 017 a parsed document breaks.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidMarkup {
    /// An `x:` directive other than `Class`, `ViewModel`, `Name`, `Key`.
    #[error("unknown directive 'x:{0}' (known: x:Class, x:ViewModel, x:Name, x:Key)")]
    UnknownDirective(String),
    /// `x:Class` / `x:ViewModel` below the root element.
    #[error("'x:{0}' is only allowed on the root element")]
    RootOnlyDirective(String),
    /// A directive whose value is a markup extension.
    #[error("'x:{0}' takes plain text")]
    DirectiveNotText(String),
    /// An `x:Class` / `x:ViewModel` value that is not a Rust type path.
    #[error("'{0}' is not a Rust type path")]
    InvalidTypePath(String),
    /// An `x:Name` that does not turn into a Rust identifier.
    #[error("x:Name '{0}' is not usable as an identifier")]
    InvalidName(String),
    #[error("x:Name '{0}' is used twice")]
    DuplicateName(String),
    /// Two different `x:Name`s that map to the same `Name` variant.
    #[error("x:Name '{second}' generates the same identifier as '{first}'")]
    NameCollision { first: String, second: String },
}
