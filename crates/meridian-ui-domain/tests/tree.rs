//! Tree structure, property storage and the change journal.

use meridian_ui_domain::layout::value::Length;
use meridian_ui_domain::properties::element::WIDTH;
use meridian_ui_domain::properties::grid::ROW;
use meridian_ui_domain::properties::{self};
use meridian_ui_domain::{
    ChangeSource, ElementKind, FixedAdvanceText, PropertyValue, Size, UiError, UiTree,
    compute_layout,
};

#[test]
fn removed_nodes_go_stale_and_slots_do_not_alias() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::StackPanel);
    let child = tree.create(ElementKind::Border);
    tree.append_child(root, child).unwrap();
    tree.remove(child).unwrap();
    assert_eq!(tree.children(root).unwrap(), &[]);
    assert_eq!(tree.kind(child), Err(UiError::StaleNode(child)));
    let reused = tree.create(ElementKind::Border);
    assert_ne!(reused, child);
    assert!(!tree.contains(child));
}

#[test]
fn removing_a_node_removes_its_subtree() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::StackPanel);
    let middle = tree.create(ElementKind::StackPanel);
    let leaf = tree.create(ElementKind::TextBlock);
    tree.append_child(root, middle).unwrap();
    tree.append_child(middle, leaf).unwrap();
    tree.remove(middle).unwrap();
    assert!(!tree.contains(leaf));
    assert_eq!(tree.len(), 1);
}

#[test]
fn attaching_rejects_cycles_and_second_parents() {
    let mut tree = UiTree::new();
    let top = tree.create(ElementKind::StackPanel);
    let below = tree.create(ElementKind::StackPanel);
    tree.append_child(top, below).unwrap();
    assert_eq!(
        tree.append_child(below, top),
        Err(UiError::WouldCycle {
            parent: below,
            child: top
        })
    );
    assert_eq!(
        tree.append_child(top, top),
        Err(UiError::WouldCycle {
            parent: top,
            child: top
        })
    );
    let other = tree.create(ElementKind::StackPanel);
    assert_eq!(
        tree.append_child(other, below),
        Err(UiError::AlreadyAttached(below))
    );
}

#[test]
fn unset_properties_read_as_defaults() {
    let mut tree = UiTree::new();
    let node = tree.create(ElementKind::Grid);
    assert_eq!(tree.get(node, WIDTH).unwrap(), Length::Auto);
    assert_eq!(tree.get(node, ROW).unwrap(), 0);
}

#[test]
fn writes_are_journaled_once_and_equal_writes_are_skipped() {
    let mut tree = UiTree::new();
    let node = tree.create(ElementKind::Border);
    tree.set(node, WIDTH, Length::Px(10.0)).unwrap();
    tree.set(node, WIDTH, Length::Px(10.0)).unwrap();
    tree.set_by_user(node, WIDTH, Length::Px(20.0)).unwrap();
    let changes = tree.drain_changes();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].source, ChangeSource::Code);
    assert_eq!(changes[1].source, ChangeSource::User);
    assert_eq!(changes[1].property, WIDTH.meta().id);
    assert_eq!(tree.drain_changes(), Vec::new());
}

#[test]
fn layout_dirty_follows_layout_affecting_writes() {
    let mut tree = UiTree::new();
    let node = tree.create(ElementKind::Border);
    compute_layout(
        &mut tree,
        node,
        Size::new(10.0, 10.0),
        &FixedAdvanceText::default(),
    )
    .unwrap();
    assert!(!tree.needs_layout());
    tree.set(node, WIDTH, Length::Px(5.0)).unwrap();
    assert!(tree.needs_layout());
}

#[test]
fn run_time_typed_access_checks_the_kind() {
    let mut tree = UiTree::new();
    let node = tree.create(ElementKind::Grid);
    let id = properties::by_name("Grid.Row").unwrap().id;
    tree.set_value(node, id, PropertyValue::Index(3)).unwrap();
    assert_eq!(tree.value_of(node, id).unwrap(), PropertyValue::Index(3));
    assert_eq!(tree.get(node, ROW).unwrap(), 3);
    assert_eq!(
        tree.set_value(node, id, PropertyValue::Bool(true)),
        Err(UiError::ValueKind(id))
    );
}

#[test]
fn stale_nodes_fail_property_access() {
    let mut tree = UiTree::new();
    let node = tree.create(ElementKind::Border);
    tree.remove(node).unwrap();
    assert_eq!(
        tree.set(node, WIDTH, Length::Px(1.0)),
        Err(UiError::StaleNode(node))
    );
    assert_eq!(tree.get(node, WIDTH), Err(UiError::StaleNode(node)));
}

#[test]
fn element_tags_round_trip_and_unknown_tags_are_none() {
    for kind in ElementKind::ALL {
        assert_eq!(ElementKind::from_tag(kind.tag()), Some(kind));
    }
    assert_eq!(ElementKind::from_tag("grid"), None);
    assert_eq!(ElementKind::from_tag("Button"), None);
}
