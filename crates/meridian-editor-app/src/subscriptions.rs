//! [`Subscriptions`]: which event topics an editor listens to.

use std::collections::HashSet;

use meridian_editor_domain::{Event, EventTopic};

/// The topics one editor subscribed to. None at first: events are sent only
/// on request.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Subscriptions {
    topics: HashSet<EventTopic>,
}

impl Subscriptions {
    /// No topics.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `topic`; subscribing twice is the same as once.
    pub fn subscribe(&mut self, topic: EventTopic) {
        self.topics.insert(topic);
    }

    /// Removes `topic`; leaving one that was never joined is harmless.
    pub fn unsubscribe(&mut self, topic: EventTopic) {
        self.topics.remove(&topic);
    }

    /// Whether `topic` is subscribed.
    #[must_use]
    pub fn contains(&self, topic: EventTopic) -> bool {
        self.topics.contains(&topic)
    }

    /// Whether `event` belongs to a subscribed topic.
    #[must_use]
    pub fn wants(&self, event: &Event) -> bool {
        self.contains(topic_of(event))
    }
}

/// The topic `event` is announced under.
fn topic_of(event: &Event) -> EventTopic {
    match event {
        Event::EntityAdded { .. } | Event::EntityRemoved { .. } => EventTopic::Entities,
        Event::ComponentChanged { .. } => EventTopic::Components,
        Event::Log { .. } => EventTopic::Log,
        Event::PlayStateChanged { .. } => EventTopic::PlayState,
    }
}
