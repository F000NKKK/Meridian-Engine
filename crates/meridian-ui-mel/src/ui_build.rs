//! `build_ui`: the generated function that creates a document's
//! `meridian-ui-domain` elements.

use std::path::Path;

use crate::UiNode;
use crate::code_writer::{ALLOW, CodeWriter};

const UI: &str = "::meridian_sdk::ui_core";

/// Renders `build_ui(tree) -> Result<(NodeId, Names), UiError>` for the
/// tree `root`: elements in document order, attributes through
/// `set_literal`, children in markup order.
pub(crate) fn render_build_ui(code: &mut CodeWriter, path: &Path, root: &UiNode) {
    code.line(
        "\n/// Creates the document's elements in `tree`; returns the root and every `x:Name`.",
    );
    code.line(ALLOW);
    code.line(format!(
        "pub(crate) fn build_ui(tree: &mut {UI}::UiTree) -> Result<({UI}::NodeId, Names), {UI}::UiError> {{"
    ));
    let mut writer = NodeWriter {
        code,
        path,
        next: 0,
        named: Vec::new(),
    };
    writer.node(root, None);
    let named = writer.named;
    code.line("    Ok((n0, Names {");
    for (field, index) in named {
        code.line(format!("        {field}: n{index},"));
    }
    code.line("    }))\n}");
}

/// Numbers the elements `n0`, `n1`, ... as it writes them.
struct NodeWriter<'a> {
    code: &'a mut CodeWriter,
    path: &'a Path,
    next: usize,
    /// `Names` field and element number of every `x:Name`.
    named: Vec<(String, usize)>,
}

impl NodeWriter<'_> {
    fn node(&mut self, node: &UiNode, parent: Option<usize>) {
        let index = self.next;
        self.next += 1;
        self.code.line(format!(
            "    let n{index} = tree.create({UI}::ElementKind::{:?});",
            node.kind
        ));
        if let Some(field) = &node.field {
            self.named.push((field.clone(), index));
        }
        for literal in &node.literals {
            self.code.line(format!(
                "    // @meui {}:{} {}",
                self.path.display(),
                literal.span,
                literal.property
            ));
            self.code.line(format!(
                "    tree.set_literal(n{index}, {}.meta().id, {:?})?;",
                literal.path.to_rust(),
                literal.text
            ));
        }
        if let Some(parent) = parent {
            self.code
                .line(format!("    tree.append_child(n{parent}, n{index})?;"));
        }
        for child in &node.children {
            self.node(child, Some(index));
        }
    }
}
