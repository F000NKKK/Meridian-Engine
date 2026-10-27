//! [`LiveUi`]: a built document that can take a newer version of itself.

use std::collections::HashMap;

use meridian_ui_core::{NodeId, PropertyId, UiError, UiTree, properties};

use super::Reload;
use crate::UiNode;

/// The document last applied to a tree, rooted at `root`. Only plain
/// calls on `UiTree` are made, so markup stays optional (ADR 018).
#[derive(Debug, Clone)]
pub struct LiveUi {
    root: NodeId,
    applied: UiNode,
}

impl LiveUi {
    /// `applied` is what built the tree at `root`: the `ui` of the
    /// `MelDocument` the compiled `build_ui` was generated from.
    pub fn new(root: NodeId, applied: UiNode) -> Self {
        Self { root, applied }
    }

    /// Makes the tree at the root match `next`. A change of shape leaves
    /// the tree alone and answers [`Reload::Restructured`]; otherwise the
    /// attributes that differ are set (or reset to the property's
    /// default when `next` no longer writes them).
    pub fn reload(&mut self, tree: &mut UiTree, next: UiNode) -> Result<Reload, UiError> {
        if !same_shape(&self.applied, &next) {
            return Ok(Reload::Restructured);
        }
        let changed = patch(tree, self.root, &self.applied, &next)?;
        self.applied = next;
        Ok(Reload::Patched { changed })
    }
}

/// Same kinds, `x:Name`s and child counts, recursively.
fn same_shape(old: &UiNode, new: &UiNode) -> bool {
    old.kind == new.kind
        && old.field == new.field
        && old.children.len() == new.children.len()
        && old
            .children
            .iter()
            .zip(&new.children)
            .all(|(old, new)| same_shape(old, new))
}

fn patch(tree: &mut UiTree, node: NodeId, old: &UiNode, new: &UiNode) -> Result<usize, UiError> {
    let before = texts(old);
    let after = texts(new);
    let mut changed = 0;
    for (id, text) in &after {
        if before.get(id) != Some(text) {
            tree.set_literal(node, *id, text)?;
            changed += 1;
        }
    }
    for id in before.keys().filter(|id| !after.contains_key(id)) {
        let meta = properties::meta(*id).ok_or(UiError::UnknownProperty(*id))?;
        tree.set_value(node, *id, (meta.default)())?;
        changed += 1;
    }
    let children = tree.children(node)?.to_vec();
    for ((child, old), new) in children.into_iter().zip(&old.children).zip(&new.children) {
        changed += patch(tree, child, old, new)?;
    }
    Ok(changed)
}

/// The attribute text of every literal of `node`; a later attribute
/// wins, as it does when the generated code sets them in order.
fn texts(node: &UiNode) -> HashMap<PropertyId, &str> {
    node.literals
        .iter()
        .map(|literal| (literal.path.id(), literal.text.as_str()))
        .collect()
}
