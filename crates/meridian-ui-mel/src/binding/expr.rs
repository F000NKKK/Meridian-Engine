//! [`Expr`]: the syntax tree of an `{x:Bind ...}` expression.
//!
//! The language is deliberately small (the same scope as Yog's `x:Bind`):
//! members and calls on the view model (`count`, `count_text()`,
//! `state.name`), type paths (`Strings::title(id)`, `Filter::All`),
//! literals (`1`, `1.5`, `true`, `'text'`), `!x`, `a == b`, `a != b` and
//! the null-conditional `a?.b`. Anything larger belongs in a view-model
//! method.

use super::compare_operator::CompareOperator;
use super::expr_error::ExprError;
use super::literal::Literal;

/// One expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    /// A member of the view model: `count`, or `count_text()` with
    /// `arguments: Some(..)`.
    Member {
        name: String,
        arguments: Option<Vec<Expr>>,
    },
    /// A Rust path with at least one `::`: an associated function
    /// (`Strings::title(id)`) or a constant / enum variant (`Filter::All`).
    Path {
        segments: Vec<String>,
        arguments: Option<Vec<Expr>>,
    },
    /// `target.name`, `target.name(..)` or, with `optional`, `target?.name`.
    Access {
        target: Box<Expr>,
        name: String,
        arguments: Option<Vec<Expr>>,
        optional: bool,
    },
    Not(Box<Expr>),
    Compare {
        left: Box<Expr>,
        operator: CompareOperator,
        right: Box<Expr>,
    },
}

impl Expr {
    /// Whether the expression names a place a `TwoWay` binding can write
    /// back to: a member path with no calls, literals or operators.
    pub fn is_assignable(&self) -> bool {
        match self {
            Self::Member { arguments, .. } => arguments.is_none(),
            Self::Access {
                target,
                arguments,
                optional,
                ..
            } => arguments.is_none() && !optional && target.is_assignable(),
            _ => false,
        }
    }

    /// Whether a `?.` occurs anywhere, making the value optional.
    pub fn is_optional(&self) -> bool {
        match self {
            Self::Literal(_) => false,
            Self::Member { arguments, .. } | Self::Path { arguments, .. } => {
                arguments.iter().flatten().any(Self::is_optional)
            }
            Self::Access {
                target,
                arguments,
                optional,
                ..
            } => {
                *optional
                    || target.is_optional()
                    || arguments.iter().flatten().any(Self::is_optional)
            }
            Self::Not(inner) => inner.is_optional(),
            Self::Compare { left, right, .. } => left.is_optional() || right.is_optional(),
        }
    }
}

impl Expr {
    /// Parses a complete expression.
    pub fn parse(text: &str) -> Result<Self, ExprError> {
        super::expr_parser::ExprParser::parse(text)
    }
}
