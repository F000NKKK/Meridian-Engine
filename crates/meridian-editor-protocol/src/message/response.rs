//! [`Response`]: what the host answers to one request.

use meridian_scene::ComponentShape;
use substrate_data::{Schema, Value};

use crate::{EntityRef, ProtocolVersion, RequestError};

/// What the host answers to one request. Variants are only ever appended.
#[derive(Debug, Clone, PartialEq, Schema)]
pub enum Response {
    /// To `Hello`: the negotiated version (the lower of the host's and the
    /// editor's), which both peers speak from now on.
    Welcome {
        /// The negotiated version.
        version: ProtocolVersion,
    },
    /// To `Describe`: `ComponentRegistry::describe()`, as it is.
    Described {
        /// Every registered component's shape, in registration order.
        components: Vec<ComponentShape>,
    },
    /// To `ListEntities`.
    Entities {
        /// Every entity.
        entities: Vec<EntityRef>,
    },
    /// To `GetComponent`.
    Component {
        /// The component's value.
        value: Value,
    },
    /// To `SpawnEntity`.
    Spawned {
        /// The new entity.
        entity: EntityRef,
    },
    /// To a request that has nothing to return and succeeded
    /// (`SetComponent`, `DespawnEntity`, `Play`, `Pause`, `Step`,
    /// `Subscribe`, `Unsubscribe`).
    Done,
    /// To any request that failed.
    Failed {
        /// Why.
        error: RequestError,
    },
}
