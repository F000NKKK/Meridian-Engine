//! Typed event queues published to and drained by stages.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Workspace-wide event bus: a frame-scoped mailbox, not a persistent log.
/// [`publish`](Self::publish) queues an event by its concrete type;
/// [`drain`](Self::drain) removes and returns every event of that type
/// published since the last drain. This is what lets subsystems
/// communicate without depending on each other directly — e.g. a future
/// `physics-domain` contact could be published here and consumed by
/// `audio-domain` for an impact sound, without either crate knowing the
/// other exists (see docs/dependency-rules.md rule 7: only `runtime-app`
/// is allowed to know about both). Events are `Send` so the bus can be
/// shared between stages as `Shared<EventSystem>` (ADR 031).
#[derive(Default)]
pub struct EventSystem {
    queues: HashMap<TypeId, Vec<Box<dyn Any + Send>>>,
}

impl std::fmt::Debug for EventSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventSystem")
            .field("queued_event_types", &self.queues.len())
            .finish()
    }
}

impl EventSystem {
    /// Creates an event bus with empty queues.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `event`, keyed by its concrete type `E`. Multiple event
    /// types don't collide — each gets its own queue.
    pub fn publish<E: Send + 'static>(&mut self, event: E) {
        self.queues
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Box::new(event));
    }

    /// Removes and returns every queued event of type `E`, in publish
    /// order. A second call before any new `publish::<E>` returns empty —
    /// this is a drain, not a peek.
    pub fn drain<E: 'static>(&mut self) -> Vec<E> {
        let Some(boxed) = self.queues.remove(&TypeId::of::<E>()) else {
            return Vec::new();
        };
        boxed
            .into_iter()
            .map(|b| *b.downcast::<E>().expect("queue keyed by TypeId::of::<E>()"))
            .collect()
    }
}
