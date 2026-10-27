//! Routing input to elements.

use super::UiTree;
use crate::{
    FocusDirection, InputEvent, InputOutcome, Key, Modifiers, NodeId, Point, PointerButton,
    UiEvent, UiEventKind, hit_test,
};

impl UiTree {
    /// Routes one input event through the tree under `root`, using the
    /// last layout, and queues the resulting [`UiEvent`]s. The UI
    /// *consumes* pointer events over any hit-test-visible element, key
    /// and text events while an element has focus, and `Tab` while any
    /// element can take focus.
    pub fn handle_input(&mut self, root: NodeId, event: InputEvent) -> InputOutcome {
        match event {
            InputEvent::PointerMoved(at) => self.pointer_moved(root, at),
            InputEvent::PointerPressed { button, at } => self.pointer_pressed(root, button, at),
            InputEvent::PointerReleased { button, at } => self.pointer_released(root, button, at),
            InputEvent::KeyPressed { key, modifiers } => self.key_pressed(root, key, modifiers),
            InputEvent::Text(text) => self.text_typed(text),
        }
    }

    /// Takes and clears the queued UI events, oldest first.
    pub fn drain_events(&mut self) -> Vec<UiEvent> {
        std::mem::take(&mut self.events)
    }

    /// The element under the pointer after the last move.
    pub fn hovered(&self) -> Option<NodeId> {
        self.pointer.hovered
    }

    /// Forgets focus, hover and press state that points at removed nodes.
    pub(super) fn forget_removed_nodes(&mut self) {
        if self.focused.is_some_and(|node| !self.contains(node)) {
            self.focused = None;
        }
        if self
            .pointer
            .hovered
            .is_some_and(|node| !self.contains(node))
        {
            self.pointer.hovered = None;
        }
        if self
            .pointer
            .pressed
            .is_some_and(|(_, node)| !self.contains(node))
        {
            self.pointer.pressed = None;
        }
    }

    fn pointer_moved(&mut self, root: NodeId, at: Point) -> InputOutcome {
        let target = hit_test(self, root, at);
        if target != self.pointer.hovered {
            if let Some(left) = self.pointer.hovered {
                self.events.push(UiEvent {
                    target: left,
                    kind: UiEventKind::PointerLeft,
                });
            }
            if let Some(entered) = target {
                self.events.push(UiEvent {
                    target: entered,
                    kind: UiEventKind::PointerEntered,
                });
            }
            self.pointer.hovered = target;
        }
        consumed_if(target.is_some())
    }

    fn pointer_pressed(&mut self, root: NodeId, button: PointerButton, at: Point) -> InputOutcome {
        let Some(target) = hit_test(self, root, at) else {
            return InputOutcome::Ignored;
        };
        self.pointer.pressed = Some((button, target));
        self.focus_nearest(target);
        self.events.push(UiEvent {
            target,
            kind: UiEventKind::PointerPressed { button, at },
        });
        InputOutcome::Consumed
    }

    fn pointer_released(&mut self, root: NodeId, button: PointerButton, at: Point) -> InputOutcome {
        let pressed = self
            .pointer
            .pressed
            .take()
            .filter(|(held, _)| *held == button);
        let target = hit_test(self, root, at);
        if let Some(target) = target {
            self.events.push(UiEvent {
                target,
                kind: UiEventKind::PointerReleased { button, at },
            });
            if pressed.is_some_and(|(_, node)| node == target) {
                self.events.push(UiEvent {
                    target,
                    kind: UiEventKind::Click { button },
                });
            }
        }
        consumed_if(target.is_some() || pressed.is_some())
    }

    fn key_pressed(&mut self, root: NodeId, key: Key, modifiers: Modifiers) -> InputOutcome {
        if key == Key::Tab {
            let direction = if modifiers.contains(Modifiers::SHIFT) {
                FocusDirection::Previous
            } else {
                FocusDirection::Next
            };
            return consumed_if(self.move_focus(root, direction).is_some());
        }
        match self.focused {
            Some(target) => {
                self.events.push(UiEvent {
                    target,
                    kind: UiEventKind::KeyPressed { key, modifiers },
                });
                InputOutcome::Consumed
            }
            None => InputOutcome::Ignored,
        }
    }

    fn text_typed(&mut self, text: String) -> InputOutcome {
        match self.focused {
            Some(target) => {
                self.events.push(UiEvent {
                    target,
                    kind: UiEventKind::Text(text),
                });
                InputOutcome::Consumed
            }
            None => InputOutcome::Ignored,
        }
    }
}

fn consumed_if(used: bool) -> InputOutcome {
    if used {
        InputOutcome::Consumed
    } else {
        InputOutcome::Ignored
    }
}
