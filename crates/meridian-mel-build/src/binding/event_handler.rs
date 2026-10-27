//! [`EventHandler`]: one `event="{x:Bind method}"` of a named element
//! (`docs/meui-events.md`).

use meridian_dsl_core::{MarkupExtension, MarkupValue, Span};
use meridian_ui_core::UiEventTag;

use super::expr::Expr;
use crate::{InvalidMarkup, NamedElement};

/// The name `method(e)` passes the event under.
const EVENT_ARGUMENT: &str = "e";

#[derive(Debug, Clone, PartialEq)]
pub struct EventHandler {
    /// The `Names` field of the element the handler sits on.
    pub field: String,
    /// The event attribute as written (`click`).
    pub attribute: String,
    pub tag: UiEventTag,
    /// The view-model method, called as `vm.method()` or, with
    /// `takes_event`, `vm.method(event)`.
    pub method: String,
    pub takes_event: bool,
    /// Where the attribute is.
    pub span: Span,
}

impl EventHandler {
    /// Reads `{x:Bind method}` or `{x:Bind method(e)}` for `tag`.
    pub fn new(
        element: &NamedElement,
        attribute: &str,
        tag: UiEventTag,
        extension: &MarkupExtension,
        span: Span,
    ) -> Result<Self, InvalidMarkup> {
        let [MarkupValue::Text(text)] = extension.positional.as_slice() else {
            return Err(InvalidMarkup::BindingArity);
        };
        if let Some((key, _)) = extension.named.first() {
            return Err(InvalidMarkup::EventHandlerArgument(key.to_string()));
        }
        let expr = Expr::parse(text).map_err(InvalidMarkup::BindingExpression)?;
        let (method, takes_event) = Self::shape(&expr)
            .ok_or_else(|| InvalidMarkup::EventHandlerShape(attribute.to_string()))?;
        Ok(Self {
            field: element.field.clone(),
            attribute: attribute.to_string(),
            tag,
            method,
            takes_event,
            span,
        })
    }

    /// `method` and `method()` -> no event; `method(e)` -> the event.
    fn shape(expr: &Expr) -> Option<(String, bool)> {
        let Expr::Member { name, arguments } = expr else {
            return None;
        };
        match arguments.as_deref() {
            None | Some([]) => Some((name.clone(), false)),
            Some(
                [
                    Expr::Member {
                        name: argument,
                        arguments: None,
                    },
                ],
            ) if argument == EVENT_ARGUMENT => Some((name.clone(), true)),
            Some(_) => None,
        }
    }

    /// The call as Rust, on the view model `vm`.
    pub fn call(&self) -> String {
        let event = if self.takes_event { "event" } else { "" };
        format!("vm.{}({event})", self.method)
    }
}
