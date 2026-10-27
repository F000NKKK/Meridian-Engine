//! Building a `UiNode` into a tree at run time: what the generated
//! `build_ui` does, from the parsed document.

use meridian_ui_core::{NodeId, UiError, UiTree};

use crate::UiNode;

/// Creates `node` and its subtree in `tree`, attributes through
/// `set_literal`; appends every `x:Name`d element to `named`.
pub(super) fn build(
    tree: &mut UiTree,
    node: &UiNode,
    named: &mut Vec<(String, NodeId)>,
) -> Result<NodeId, UiError> {
    let id = tree.create(node.kind);
    if let Some(field) = &node.field {
        named.push((field.clone(), id));
    }
    for literal in &node.literals {
        tree.set_literal(id, literal.path.id(), &literal.text)?;
    }
    for child in &node.children {
        let child = build(tree, child, named)?;
        tree.append_child(id, child)?;
    }
    Ok(id)
}
