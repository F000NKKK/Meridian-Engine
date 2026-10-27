//! `build_ui`: the generated function that creates a document's
//! `meridian-ui-core` elements.

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
        "pub fn build_ui(tree: &mut {UI}::UiTree) -> Result<({UI}::NodeId, Names), {UI}::UiError> {{"
    ));
    let mut named = Vec::new();
    let mut next = 0;
    render_node(code, path, root, None, &mut next, &mut named);
    code.line("    Ok((n0, Names {");
    for (field, index) in named {
        code.line(format!("        {field}: n{index},"));
    }
    code.line("    }))\n}");
}

fn render_node(
    code: &mut CodeWriter,
    path: &Path,
    node: &UiNode,
    parent: Option<usize>,
    next: &mut usize,
    named: &mut Vec<(String, usize)>,
) {
    let index = *next;
    *next += 1;
    code.line(format!(
        "    let n{index} = tree.create({UI}::ElementKind::{:?});",
        node.kind
    ));
    if let Some(field) = &node.field {
        named.push((field.clone(), index));
    }
    for literal in &node.literals {
        code.line(format!(
            "    // @mel {}:{} {}",
            path.display(),
            literal.span,
            literal.property
        ));
        code.line(format!(
            "    tree.set_literal(n{index}, {}.meta().id, {:?})?;",
            literal.path.to_rust(),
            literal.text
        ));
    }
    if let Some(parent) = parent {
        code.line(format!("    tree.append_child(n{parent}, n{index})?;"));
    }
    for child in &node.children {
        render_node(code, path, child, Some(index), next, named);
    }
}
