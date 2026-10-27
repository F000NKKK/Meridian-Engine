//! Rendering a [`Expr`] as Rust source over a view model `vm`.

use super::compare_operator::CompareOperator;
use super::expr::Expr;
use super::literal::Literal;

impl Expr {
    /// The expression as Rust, with view-model members read from `vm`.
    /// A `?.` renders as `.as_ref()?.`, so an optional expression must
    /// sit in a context returning `Option` (see [`Self::is_optional`]).
    /// Call arguments that name a place (`id`, `item.id`) are passed by
    /// reference, as in Yog's `x:Bind`.
    pub fn to_rust(&self) -> String {
        match self {
            Self::Literal(literal) => literal.to_rust(),
            Self::Member { name, arguments } => {
                format!("vm.{name}{}", render_arguments(arguments.as_deref()))
            }
            Self::Path {
                segments,
                arguments,
            } => format!(
                "{}{}",
                segments.join("::"),
                render_arguments(arguments.as_deref())
            ),
            Self::Access {
                target,
                name,
                arguments,
                optional,
            } => {
                let step = if *optional { ".as_ref()?." } else { "." };
                let arguments = render_arguments(arguments.as_deref());
                format!("{}{step}{name}{arguments}", target.to_rust())
            }
            Self::Not(inner) => format!("!({})", inner.to_rust()),
            Self::Compare {
                left,
                operator,
                right,
            } => {
                let operator = match operator {
                    CompareOperator::Equal => "==",
                    CompareOperator::NotEqual => "!=",
                };
                format!("({} {operator} {})", left.to_rust(), right.to_rust())
            }
        }
    }
}

impl Literal {
    fn to_rust(&self) -> String {
        match self {
            Self::Integer(text) | Self::Float(text) => text.clone(),
            Self::Bool(value) => value.to_string(),
            Self::Text(text) => format!("{text:?}"),
        }
    }
}

fn render_arguments(arguments: Option<&[Expr]>) -> String {
    let Some(arguments) = arguments else {
        return String::new();
    };
    let rendered: Vec<String> = arguments
        .iter()
        .map(|argument| {
            let code = argument.to_rust();
            if argument.is_assignable() {
                format!("&{code}")
            } else {
                code
            }
        })
        .collect();
    format!("({})", rendered.join(", "))
}
