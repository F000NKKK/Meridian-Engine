//! [`ResourceProblem`]: what is wrong with a `.mres` file.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResourceProblem {
    /// The text does not parse as Substrate Data.
    #[error("{0}")]
    Syntax(String),
    #[error("a resource set is an object of named entries")]
    NotAnObject,
    /// A set name (the file stem) that is not an identifier.
    #[error("'{0}' is not a usable set name (letters, digits and '_', starting with a letter)")]
    InvalidSetName(String),
    #[error("resource set '{0}' is defined twice")]
    DuplicateSet(String),
    #[error("'{0}' is not a usable resource name (dotted identifiers: 'title', 'screen.title')")]
    InvalidName(String),
    /// Two names that generate the same constant.
    #[error("resources '{second}' and '{first}' generate the same constant")]
    NameCollision { first: String, second: String },
    #[error("resource '{0}' has no 'value'")]
    MissingValue(String),
    #[error("resource '{entry}' has an unknown field '{field}' (known: value, type, comment)")]
    UnknownField { entry: String, field: String },
    #[error(
        "resource '{entry}' has an unknown type '{found}' (known: string, number, bool, color)"
    )]
    UnknownType { entry: String, found: String },
    /// A value of the wrong kind for the entry's type.
    #[error("resource '{entry}' must be {expected}")]
    WrongValue {
        entry: String,
        expected: &'static str,
    },
    #[error("resource '{entry}': '{found}' is not a color (#RRGGBB or #AARRGGBB)")]
    BadColor { entry: String, found: String },
    /// A text that spells a reference or bytes instead of plain text.
    #[error("resource '{entry}': {message}")]
    NotPlainText { entry: String, message: String },
}
