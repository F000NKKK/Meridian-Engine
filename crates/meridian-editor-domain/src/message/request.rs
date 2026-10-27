//! [`Request`]: what the editor asks of the host.

use substrate_data::{Schema, Value};

use crate::{ComponentRef, EntityRef, EventTopic, ProtocolVersion};

/// What the editor asks of the host. Variants are only ever appended.
#[derive(Debug, Clone, PartialEq, Schema)]
pub enum Request {
    /// The first request of a connection: the protocol version the editor
    /// speaks. Answered with [`crate::Response::Welcome`] (the negotiated version), or
    /// [`crate::RequestError::VersionMismatch`] when the editor is newer.
    Hello {
        /// The editor's version.
        version: ProtocolVersion,
    },
    /// What components the game has and what each looks like.
    Describe,
    /// Every entity of the running game.
    ListEntities,
    /// One component of one entity, as a Substrate Data value.
    GetComponent {
        /// The entity.
        entity: EntityRef,
        /// Its component.
        component: ComponentRef,
    },
    /// Replaces one component of one entity. The host reads `value` with
    /// the component's schema and, when the entity comes from a text
    /// document, writes it back with `substrate_data::patch::patch`, so
    /// hand-written comments and layout survive.
    SetComponent {
        /// The entity.
        entity: EntityRef,
        /// Its component.
        component: ComponentRef,
        /// The component's new value.
        value: Value,
    },
    /// Spawns an entity with no components.
    SpawnEntity,
    /// Despawns an entity.
    DespawnEntity {
        /// The entity.
        entity: EntityRef,
    },
    /// Runs the simulation.
    Play,
    /// Stops the simulation, keeping its state.
    Pause,
    /// Runs one simulation step; pauses afterwards.
    Step,
    /// Starts sending the events of `topic`.
    Subscribe {
        /// Which events.
        topic: EventTopic,
    },
    /// Stops sending the events of `topic`.
    Unsubscribe {
        /// Which events.
        topic: EventTopic,
    },
}
