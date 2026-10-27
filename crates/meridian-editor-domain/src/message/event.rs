//! [`Event`]: what the host announces to subscribers.

use substrate_data::{Schema, Value};

use crate::{ComponentRef, EntityRef, LogLevel, PlayState};

/// What the host announces, without being asked, to an editor subscribed to
/// the event's [`crate::EventTopic`]. Variants are only ever appended.
#[derive(Debug, Clone, PartialEq, Schema)]
pub enum Event {
    /// An entity appeared (topic `Entities`).
    EntityAdded {
        /// The entity.
        entity: EntityRef,
    },
    /// An entity went away (topic `Entities`).
    EntityRemoved {
        /// The entity.
        entity: EntityRef,
    },
    /// A component changed value (topic `Components`).
    ComponentChanged {
        /// The entity.
        entity: EntityRef,
        /// Its component.
        component: ComponentRef,
        /// The new value.
        value: Value,
    },
    /// The game logged a line (topic `Log`).
    Log {
        /// How important.
        level: LogLevel,
        /// What it said.
        message: String,
    },
    /// The simulation started or stopped (topic `PlayState`).
    PlayStateChanged {
        /// Where it stands now.
        state: PlayState,
    },
}
