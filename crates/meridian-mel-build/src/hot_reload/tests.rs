//! Tests for [`LiveUi::reload`].

use meridian_dsl_core::parse;
use meridian_ui_core::layout::value::Length;
use meridian_ui_core::{NodeId, UiTree, properties};

use super::*;
use crate::{ResourceSets, UiNode};

fn node(source: &str) -> UiNode {
    let root = parse(source).unwrap();
    UiNode::from_element(&root, &[], &ResourceSets::default()).unwrap()
}

/// What the generated `build_ui` does.
fn build(tree: &mut UiTree, ui: &UiNode) -> NodeId {
    let id = tree.create(ui.kind);
    for literal in &ui.literals {
        tree.set_literal(id, literal.path.id(), &literal.text)
            .unwrap();
    }
    for child in &ui.children {
        let child = build(tree, child);
        tree.append_child(id, child).unwrap();
    }
    id
}

fn live(source: &str) -> (UiTree, NodeId, LiveUi) {
    let ui = node(source);
    let mut tree = UiTree::new();
    let root = build(&mut tree, &ui);
    (tree, root, LiveUi::new(root, ui))
}

const BEFORE: &str = r#"<StackPanel width="200">
    <TextBlock text="Hi" font_size="12" />
</StackPanel>"#;

#[test]
fn changed_attributes_are_set_in_place() {
    let (mut tree, root, mut live) = live(BEFORE);
    let text = tree.children(root).unwrap()[0];
    let after = BEFORE.replace("Hi", "Hello").replace("200", "300");
    let outcome = live.reload(&mut tree, node(&after)).unwrap();
    assert_eq!(outcome, Reload::Patched { changed: 2 });
    assert_eq!(
        tree.get(root, properties::element::WIDTH).unwrap(),
        Length::Px(300.0)
    );
    assert_eq!(
        tree.get(text, properties::text_block::TEXT).unwrap(),
        "Hello"
    );
    assert_eq!(tree.children(root).unwrap()[0], text);
}

#[test]
fn an_unchanged_document_changes_nothing() {
    let (mut tree, _, mut live) = live(BEFORE);
    tree.drain_changes();
    let outcome = live.reload(&mut tree, node(BEFORE)).unwrap();
    assert_eq!(outcome, Reload::Patched { changed: 0 });
    assert!(tree.drain_changes().is_empty());
}

#[test]
fn a_removed_attribute_goes_back_to_the_default() {
    let (mut tree, root, mut live) = live(BEFORE);
    let after = BEFORE.replace(r#" width="200""#, "");
    let outcome = live.reload(&mut tree, node(&after)).unwrap();
    assert_eq!(outcome, Reload::Patched { changed: 1 });
    let default = (properties::element::WIDTH.meta().default)();
    assert_eq!(
        tree.value_of(root, properties::element::WIDTH.meta().id)
            .unwrap(),
        default
    );
}

#[test]
fn a_new_shape_leaves_the_tree_alone() {
    let (mut tree, root, mut live) = live(BEFORE);
    tree.drain_changes();
    let after = BEFORE
        .replace("<TextBlock", "<StackPanel")
        .replace(r#" text="Hi" font_size="12" />"#, " />");
    assert_eq!(
        live.reload(&mut tree, node(&after)).unwrap(),
        Reload::Restructured
    );
    let extra = r#"<StackPanel width="200"><TextBlock text="Hi" font_size="12" /><TextBlock text="2" /></StackPanel>"#;
    assert_eq!(
        live.reload(&mut tree, node(extra)).unwrap(),
        Reload::Restructured
    );
    assert!(tree.drain_changes().is_empty());
    assert_eq!(
        tree.get(root, properties::element::WIDTH).unwrap(),
        Length::Px(200.0)
    );
    // Still the old document: a matching reload patches.
    assert_eq!(
        live.reload(&mut tree, node(BEFORE)).unwrap(),
        Reload::Patched { changed: 0 }
    );
}
