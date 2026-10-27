//! [`InvalidMarkup`]: a rule of ADR 017 that a parsed `.mel` document breaks.

use std::fmt;

/// A rule of ADR 017 a parsed document breaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidMarkup {
    /// An `x:` directive other than `Class`, `ViewModel`, `Name`, `Key`.
    UnknownDirective(String),
    /// `x:Class` / `x:ViewModel` below the root element.
    RootOnlyDirective(String),
    /// A directive whose value is a markup extension.
    DirectiveNotText(String),
    /// An `x:Class` / `x:ViewModel` value that is not a Rust type path.
    InvalidTypePath(String),
    /// An `x:Name` that does not turn into a Rust identifier.
    InvalidName(String),
    DuplicateName(String),
    /// Two different `x:Name`s that map to the same `Name` variant.
    NameCollision {
        first: String,
        second: String,
    },
}

impl fmt::Display for InvalidMarkup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownDirective(name) => write!(
                f,
                "unknown directive 'x:{name}' (known: x:Class, x:ViewModel, x:Name, x:Key)"
            ),
            Self::RootOnlyDirective(name) => {
                write!(f, "'x:{name}' is only allowed on the root element")
            }
            Self::DirectiveNotText(name) => write!(f, "'x:{name}' takes plain text"),
            Self::InvalidTypePath(path) => write!(f, "'{path}' is not a Rust type path"),
            Self::InvalidName(name) => write!(f, "x:Name '{name}' is not usable as an identifier"),
            Self::DuplicateName(name) => write!(f, "x:Name '{name}' is used twice"),
            Self::NameCollision { first, second } => write!(
                f,
                "x:Name '{second}' generates the same identifier as '{first}'"
            ),
        }
    }
}
