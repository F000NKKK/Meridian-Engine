//! The type check a [`PropertyBinding`] contributes to the generated
//! `type_check_bindings`: code that compiles only if the expression fits
//! the view model.

use super::mode::Mode;
use super::property_binding::PropertyBinding;

impl PropertyBinding {
    /// Statements, one per line, that read the binding from `vm`
    /// (`&mut ViewModel`). A `TwoWay` path is borrowed mutably, a
    /// `BindBack` must name a view-model method, and a `?.` expression is
    /// read inside a closure returning `Option`.
    pub fn type_check(&self) -> Vec<String> {
        let expr = self.expr.to_rust();
        let mut lines = Vec::new();
        if self.expr.is_optional() {
            lines.push(format!(
                "(|vm: &ViewModel| -> Option<()> {{ let _value = &({expr}); Some(()) }})(vm);"
            ));
        } else {
            lines.push(format!("let _value = &({expr});"));
        }
        if let Some(fallback) = &self.fallback {
            lines.push(format!("let _fallback = &({});", fallback.to_rust()));
        }
        match (&self.bind_back, self.mode) {
            (Some(method), _) => lines.push(format!("let _bind_back = ViewModel::{method};")),
            (None, Mode::TwoWay) => lines.push(format!("let _target = &mut ({expr});")),
            (None, _) => {}
        }
        lines
    }
}
