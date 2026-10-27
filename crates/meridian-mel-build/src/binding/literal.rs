//! [`Literal`]: a constant in an `x:Bind` expression.

/// `1`, `1.5`, `true`, `'text'`; numbers keep their spelling.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(String),
    Float(String),
    Bool(bool),
    Text(String),
}
