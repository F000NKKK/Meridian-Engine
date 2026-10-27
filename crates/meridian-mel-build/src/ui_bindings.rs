//! [`UiBindings`]: the generated glue between a view model and a
//! `meridian-ui-core` tree (`Names`, `apply_*_bindings`,
//! `write_back_bindings`).

use crate::code_writer::{ALLOW, CodeWriter};
use crate::{BindingMode, MelDocument, PropertyBinding};

const UI: &str = "::meridian_sdk::ui_core";

/// Renders the UI half of one [`MelDocument`]; nothing for a document
/// without `{x:Bind}`.
pub(crate) struct UiBindings<'a> {
    document: &'a MelDocument,
}

impl<'a> UiBindings<'a> {
    pub(crate) fn new(document: &'a MelDocument) -> Self {
        Self { document }
    }

    pub(crate) fn render(&self, code: &mut CodeWriter) {
        if self.document.bindings.is_empty() {
            return;
        }
        self.render_names(code);
        self.render_apply(code, "apply_one_time_bindings", |binding| {
            binding.mode == BindingMode::OneTime
        });
        self.render_apply(
            code,
            "apply_bindings",
            PropertyBinding::is_applied_repeatedly,
        );
        self.render_write_back(code);
    }

    fn marker(&self, code: &mut CodeWriter, indent: &str, binding: &PropertyBinding) {
        code.line(format!(
            "{indent}// @mel {}:{} {}",
            self.document.path.display(),
            binding.span,
            binding.property
        ));
    }

    fn render_names(&self, code: &mut CodeWriter) {
        code.line(
            "\n/// The element of every `x:Name`; filled in by the code that builds the tree.",
        );
        code.line(ALLOW);
        code.line("#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
        code.line("pub struct Names {");
        for named in &self.document.names {
            code.line(format!("    pub {}: {UI}::NodeId,", named.field));
        }
        code.line("}");
    }

    fn render_apply(
        &self,
        code: &mut CodeWriter,
        name: &str,
        select: impl Fn(&PropertyBinding) -> bool,
    ) {
        let bindings: Vec<_> = self
            .document
            .bindings
            .iter()
            .filter(|b| select(b))
            .collect();
        if bindings.is_empty() {
            return;
        }
        code.line(format!(
            "\n/// Writes the view model's values to the elements (`{name}`)."
        ));
        code.line(ALLOW);
        code.line(format!(
            "pub fn {name}(vm: &ViewModel, tree: &mut {UI}::UiTree, names: &Names) -> Result<(), {UI}::UiError> {{"
        ));
        for binding in bindings {
            self.marker(code, "    ", binding);
            for statement in binding.apply_statements() {
                code.line(format!("    {statement}"));
            }
        }
        code.line("    Ok(())\n}");
    }

    fn render_write_back(&self, code: &mut CodeWriter) {
        let two_way: Vec<_> = self
            .document
            .bindings
            .iter()
            .filter_map(|binding| Some((binding, binding.write_back_statements()?)))
            .collect();
        if two_way.is_empty() {
            return;
        }
        code.line("\n/// Moves the user's edits of `TwoWay` bindings into the view model.");
        code.line("/// Drains the tree's change journal: call it once per frame.");
        code.line(ALLOW);
        code.line(format!(
            "pub fn write_back_bindings(vm: &mut ViewModel, tree: &mut {UI}::UiTree, names: &Names) -> Result<(), {UI}::UiError> {{"
        ));
        code.line("    for change in tree.drain_changes() {");
        code.line(format!(
            "        if change.source != {UI}::ChangeSource::User {{"
        ));
        code.line("            continue;\n        }");
        for (binding, statements) in two_way {
            self.marker(code, "        ", binding);
            code.line(format!(
                "        if change.node == names.{} && change.property == {}.meta().id {{",
                binding.field,
                binding.path.to_rust()
            ));
            for statement in statements {
                code.line(format!("            {statement}"));
            }
            code.line("        }");
        }
        code.line("    }\n    Ok(())\n}");
    }
}
