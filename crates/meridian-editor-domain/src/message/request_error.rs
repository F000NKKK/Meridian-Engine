//! [`RequestError`]: why a request failed.

use meridian_data_substrate::ComponentName;
use substrate_data::{Diagnostic, Schema};

use crate::{EntityRef, ProtocolVersion};

/// Why a request failed — typed, never a message to parse. Variants are
/// only ever appended.
#[derive(Debug, Clone, PartialEq, Schema)]
pub enum RequestError {
    /// `Hello` came from an editor newer than the host.
    VersionMismatch {
        /// What the host speaks.
        host: ProtocolVersion,
        /// What the editor speaks.
        client: ProtocolVersion,
    },
    /// A request other than `Hello` came first on the connection.
    NotGreeted,
    /// The entity doesn't exist (any more).
    NoSuchEntity {
        /// What was asked for.
        entity: EntityRef,
    },
    /// The host has no component of that name registered.
    NoSuchComponent {
        /// What was asked for.
        component: ComponentName,
    },
    /// The entity exists but has no such component.
    ComponentMissing {
        /// The entity.
        entity: EntityRef,
        /// The component it lacks.
        component: ComponentName,
    },
    /// A `value` that is not of the component's shape.
    InvalidValue {
        /// Every problem, with its path in the value.
        diagnostics: Vec<Diagnostic>,
    },
    /// The request isn't one this host knows (a newer editor's, or one the
    /// negotiated version doesn't include).
    Unsupported,
}
