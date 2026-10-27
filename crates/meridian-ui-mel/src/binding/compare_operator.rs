//! [`CompareOperator`]: `==` or `!=` in an `x:Bind` expression.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOperator {
    Equal,
    NotEqual,
}
