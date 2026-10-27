//! [`PropertyBinding`]: one `property="{x:Bind ...}"` of a named element.

use meridian_dsl_core::{MarkupExtension, MarkupValue, Span};

use super::argument::Argument;
use super::expr::Expr;
use super::mode::Mode;
use crate::InvalidMarkup;

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyBinding {
    /// The `Name` variant of the element the binding sits on.
    pub element: String,
    /// The bound attribute as written (`text`, `Grid.Row`).
    pub property: String,
    pub expr: Expr,
    pub mode: Mode,
    /// `BindBack=method`: called with the new value instead of writing
    /// `expr` back.
    pub bind_back: Option<String>,
    /// `Fallback=expr`: the value when a `?.` meets `None`.
    pub fallback: Option<Expr>,
    /// Where the attribute is.
    pub span: Span,
}

impl PropertyBinding {
    /// Reads `{x:Bind expr, Mode=.., BindBack=.., Fallback=..}`.
    pub fn new(
        element: &str,
        property: &str,
        extension: &MarkupExtension,
        span: Span,
    ) -> Result<Self, InvalidMarkup> {
        let [MarkupValue::Text(expr)] = extension.positional.as_slice() else {
            return Err(InvalidMarkup::BindingArity);
        };
        let mut binding = Self {
            element: element.to_string(),
            property: property.to_string(),
            expr: Expr::parse(expr).map_err(InvalidMarkup::BindingExpression)?,
            mode: Mode::default(),
            bind_back: None,
            fallback: None,
            span,
        };
        for (key, value) in &extension.named {
            binding.set_argument(&key.to_string(), value)?;
        }
        if binding.mode == Mode::TwoWay
            && binding.bind_back.is_none()
            && !binding.expr.is_assignable()
        {
            return Err(InvalidMarkup::TwoWayNeedsTarget);
        }
        if binding.fallback.is_some() && !binding.expr.is_optional() {
            return Err(InvalidMarkup::FallbackWithoutOptional);
        }
        Ok(binding)
    }

    fn set_argument(&mut self, key: &str, value: &MarkupValue) -> Result<(), InvalidMarkup> {
        let invalid = || InvalidMarkup::InvalidBindingArgument(key.to_string());
        let text = value.as_text().ok_or_else(invalid)?;
        let argument = Argument::parse(key)
            .ok_or_else(|| InvalidMarkup::UnknownBindingArgument(key.to_string()))?;
        match argument {
            Argument::Mode => self.mode = Mode::parse(text).ok_or_else(invalid)?,
            Argument::BindBack => {
                let is_name = text.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
                    && text
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
                if !is_name {
                    return Err(invalid());
                }
                self.bind_back = Some(text.to_string());
            }
            Argument::Fallback => {
                let fallback = Expr::parse(text).map_err(InvalidMarkup::BindingExpression)?;
                self.fallback = Some(fallback);
            }
        }
        Ok(())
    }
}
