//! The Rust a [`PropertyBinding`] contributes to the generated
//! `apply_*_bindings` and `write_back_bindings` (ADR 018 section 5).

use super::mode::Mode;
use super::property_binding::PropertyBinding;

/// An owned copy of a value read from the view model; the target type is
/// the property's, so a mismatch is a compile error at the `// @meui` line.
fn owned(expr: &str) -> String {
    format!("::core::convert::Into::into(::core::clone::Clone::clone(&({expr})))")
}

impl PropertyBinding {
    /// Whether the binding is written to the element on every
    /// `apply_bindings`, as opposed to once.
    pub fn is_applied_repeatedly(&self) -> bool {
        self.mode != Mode::OneTime
    }

    /// Statements that write the view model's value to the element:
    /// `vm: &ViewModel`, `tree: &mut UiTree`, `names: &Names`. A `?.` that
    /// meets `None` writes the `Fallback`, or nothing without one.
    pub fn apply_statements(&self) -> Vec<String> {
        let target = format!("names.{}, {}", self.field, self.path.to_rust());
        let write = |value: String| format!("tree.set({target}, {value})?");
        if !self.expr.is_optional() {
            return vec![format!("{};", write(owned(&self.expr.to_rust())))];
        }
        let none = self.fallback.as_ref().map_or_else(
            || "{}".to_string(),
            |fallback| write(owned(&fallback.to_rust())),
        );
        vec![
            format!(
                "match (|| Some(::core::clone::Clone::clone(&({}))))() {{",
                self.expr.to_rust()
            ),
            format!(
                "    Some(value) => {},",
                write("::core::convert::Into::into(value)".into())
            ),
            format!("    None => {none},"),
            "}".to_string(),
        ]
    }

    /// For a `TwoWay` binding, the statements that move the element's new
    /// value into the view model (`vm: &mut ViewModel`), run for a user
    /// change of this element's property.
    pub fn write_back_statements(&self) -> Option<Vec<String>> {
        if self.mode != Mode::TwoWay {
            return None;
        }
        let read = format!("tree.get(names.{}, {})?", self.field, self.path.to_rust());
        let target = match &self.bind_back {
            Some(method) => format!("vm.{method}(::core::convert::Into::into(value));"),
            None => format!(
                "{} = ::core::convert::Into::into(value);",
                self.expr.to_rust()
            ),
        };
        Some(vec![format!("let value = {read};"), target])
    }
}
