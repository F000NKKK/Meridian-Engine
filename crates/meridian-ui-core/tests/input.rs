//! Focus, pointer routing and the events code-behind handles.

mod support;

use meridian_ui_core::layout::value::Visibility;
use meridian_ui_core::properties::element::VISIBILITY;
use meridian_ui_core::properties::interaction::{FOCUSABLE, IS_HIT_TEST_VISIBLE};
use meridian_ui_core::{
    ElementKind, FocusDirection, InputEvent, InputOutcome, Key, Modifiers, NodeId, Point,
    PointerButton, UiError, UiEvent, UiEventKind, UiTree,
};
use support::{block, layout};

/// A vertical stack of `count` 100x20 focusable blocks, laid out.
fn column(tree: &mut UiTree, count: usize) -> (NodeId, Vec<NodeId>) {
    let root = tree.create(ElementKind::StackPanel);
    let blocks: Vec<NodeId> = (0..count)
        .map(|_| {
            let node = block(tree, root, 100.0, 20.0);
            tree.set(node, FOCUSABLE, true).unwrap();
            node
        })
        .collect();
    layout(tree, root, 100.0, 200.0);
    (root, blocks)
}

fn press(tree: &mut UiTree, root: NodeId, x: f32, y: f32) -> InputOutcome {
    tree.handle_input(
        root,
        InputEvent::PointerPressed {
            button: PointerButton::Primary,
            at: Point::new(x, y),
        },
    )
}

fn release(tree: &mut UiTree, root: NodeId, x: f32, y: f32) -> InputOutcome {
    tree.handle_input(
        root,
        InputEvent::PointerReleased {
            button: PointerButton::Primary,
            at: Point::new(x, y),
        },
    )
}

fn kinds(events: &[UiEvent]) -> Vec<(NodeId, UiEventKind)> {
    events
        .iter()
        .map(|event| (event.target, event.kind.clone()))
        .collect()
}

#[test]
fn tab_walks_focusable_elements_in_document_order_and_wraps() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 3);
    assert_eq!(tree.move_focus(root, FocusDirection::Next), Some(nodes[0]));
    assert_eq!(tree.move_focus(root, FocusDirection::Next), Some(nodes[1]));
    assert_eq!(
        tree.move_focus(root, FocusDirection::Previous),
        Some(nodes[0])
    );
    assert_eq!(
        tree.move_focus(root, FocusDirection::Previous),
        Some(nodes[2])
    );
    assert_eq!(tree.move_focus(root, FocusDirection::Next), Some(nodes[0]));
}

#[test]
fn tab_key_moves_focus_and_shift_tab_goes_back() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 2);
    let tab = |modifiers| InputEvent::KeyPressed {
        key: Key::Tab,
        modifiers,
    };
    assert_eq!(
        tree.handle_input(root, tab(Modifiers::NONE)),
        InputOutcome::Consumed
    );
    assert_eq!(tree.focused(), Some(nodes[0]));
    assert_eq!(
        tree.handle_input(root, tab(Modifiers::SHIFT)),
        InputOutcome::Consumed
    );
    assert_eq!(tree.focused(), Some(nodes[1]));
}

#[test]
fn tab_with_nothing_focusable_is_ignored() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::StackPanel);
    block(&mut tree, root, 10.0, 10.0);
    let tab = InputEvent::KeyPressed {
        key: Key::Tab,
        modifiers: Modifiers::NONE,
    };
    assert_eq!(tree.handle_input(root, tab), InputOutcome::Ignored);
    assert_eq!(tree.focused(), None);
}

#[test]
fn hidden_and_collapsed_elements_cannot_take_focus() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 3);
    tree.set(nodes[0], VISIBILITY, Visibility::Hidden).unwrap();
    tree.set(nodes[1], VISIBILITY, Visibility::Collapsed)
        .unwrap();
    assert_eq!(tree.focus(nodes[0]), Err(UiError::NotFocusable(nodes[0])));
    assert_eq!(tree.move_focus(root, FocusDirection::Next), Some(nodes[2]));
    tree.set(root, VISIBILITY, Visibility::Hidden).unwrap();
    assert_eq!(tree.focus(nodes[2]), Err(UiError::NotFocusable(nodes[2])));
}

#[test]
fn focus_changes_raise_lost_then_gained() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 2);
    tree.focus(nodes[0]).unwrap();
    tree.drain_events();
    tree.move_focus(root, FocusDirection::Next);
    assert_eq!(
        kinds(&tree.drain_events()),
        vec![
            (nodes[0], UiEventKind::FocusLost),
            (nodes[1], UiEventKind::FocusGained),
        ]
    );
    tree.focus(nodes[1]).unwrap();
    assert!(tree.drain_events().is_empty());
}

#[test]
fn click_needs_press_and_release_on_the_same_element() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 2);
    assert_eq!(press(&mut tree, root, 5.0, 5.0), InputOutcome::Consumed);
    assert_eq!(release(&mut tree, root, 5.0, 5.0), InputOutcome::Consumed);
    let events = kinds(&tree.drain_events());
    assert!(events.contains(&(
        nodes[0],
        UiEventKind::Click {
            button: PointerButton::Primary
        }
    )));
    press(&mut tree, root, 5.0, 5.0);
    release(&mut tree, root, 5.0, 25.0);
    let events = kinds(&tree.drain_events());
    assert!(
        !events
            .iter()
            .any(|(_, kind)| matches!(kind, UiEventKind::Click { .. }))
    );
}

#[test]
fn pressing_focuses_the_nearest_focusable_ancestor_or_self() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 1);
    let inner = tree.create(ElementKind::Border);
    tree.append_child(nodes[0], inner).unwrap();
    layout(&mut tree, root, 100.0, 200.0);
    press(&mut tree, root, 5.0, 5.0);
    assert_eq!(tree.focused(), Some(nodes[0]));
    assert_eq!(tree.ancestors(inner), vec![nodes[0], root]);
}

#[test]
fn pointer_outside_every_element_is_ignored() {
    let mut tree = UiTree::new();
    let (root, _) = column(&mut tree, 1);
    assert_eq!(press(&mut tree, root, 500.0, 500.0), InputOutcome::Ignored);
    assert_eq!(
        release(&mut tree, root, 500.0, 500.0),
        InputOutcome::Ignored
    );
    assert!(tree.drain_events().is_empty());
}

#[test]
fn hover_raises_leave_before_enter() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 2);
    let moved = |x, y| InputEvent::PointerMoved(Point::new(x, y));
    assert_eq!(
        tree.handle_input(root, moved(5.0, 5.0)),
        InputOutcome::Consumed
    );
    assert_eq!(tree.hovered(), Some(nodes[0]));
    tree.handle_input(root, moved(6.0, 6.0));
    tree.handle_input(root, moved(5.0, 25.0));
    assert_eq!(
        kinds(&tree.drain_events()),
        vec![
            (nodes[0], UiEventKind::PointerEntered),
            (nodes[0], UiEventKind::PointerLeft),
            (nodes[1], UiEventKind::PointerEntered),
        ]
    );
}

#[test]
fn hit_test_invisible_elements_let_the_pointer_through() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 1);
    tree.set(nodes[0], IS_HIT_TEST_VISIBLE, false).unwrap();
    tree.set(root, IS_HIT_TEST_VISIBLE, false).unwrap();
    assert_eq!(press(&mut tree, root, 5.0, 5.0), InputOutcome::Ignored);
}

#[test]
fn keys_and_text_go_to_the_focused_element_only() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 1);
    let key = InputEvent::KeyPressed {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    };
    assert_eq!(tree.handle_input(root, key.clone()), InputOutcome::Ignored);
    assert_eq!(
        tree.handle_input(root, InputEvent::Text("a".into())),
        InputOutcome::Ignored
    );
    tree.focus(nodes[0]).unwrap();
    tree.drain_events();
    assert_eq!(tree.handle_input(root, key), InputOutcome::Consumed);
    assert_eq!(
        tree.handle_input(root, InputEvent::Text("é".into())),
        InputOutcome::Consumed
    );
    assert_eq!(
        kinds(&tree.drain_events()),
        vec![
            (
                nodes[0],
                UiEventKind::KeyPressed {
                    key: Key::Enter,
                    modifiers: Modifiers::NONE
                }
            ),
            (nodes[0], UiEventKind::Text("é".into())),
        ]
    );
}

#[test]
fn removing_nodes_drops_focus_hover_and_press_state() {
    let mut tree = UiTree::new();
    let (root, nodes) = column(&mut tree, 1);
    press(&mut tree, root, 5.0, 5.0);
    tree.handle_input(root, InputEvent::PointerMoved(Point::new(5.0, 5.0)));
    assert_eq!(tree.focused(), Some(nodes[0]));
    tree.remove(nodes[0]).unwrap();
    assert_eq!(tree.focused(), None);
    assert_eq!(tree.hovered(), None);
    tree.drain_events();
    release(&mut tree, root, 5.0, 5.0);
    let events = kinds(&tree.drain_events());
    assert!(
        !events
            .iter()
            .any(|(_, kind)| matches!(kind, UiEventKind::Click { .. }))
    );
}

#[test]
fn modifiers_combine() {
    let both = Modifiers::SHIFT.union(Modifiers::CONTROL);
    assert!(both.contains(Modifiers::SHIFT) && both.contains(Modifiers::CONTROL));
    assert!(!both.contains(Modifiers::ALT));
    assert!(Modifiers::NONE.contains(Modifiers::NONE));
}
