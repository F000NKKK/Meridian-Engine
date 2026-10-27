//! `{x:Bind ...}` bindings: the expression language, its Rust rendering
//! and the binding declarations collected from a document (ADR 017).

mod argument;
mod compare_operator;
mod expr;
mod expr_error;
mod expr_parser;
mod literal;
mod mode;
mod property_binding;
mod render;
mod type_check;

pub use compare_operator::CompareOperator;
pub use expr::Expr;
pub use expr_error::{ExprError, ExprErrorKind};
pub use literal::Literal;
pub use mode::Mode;
pub use property_binding::PropertyBinding;
