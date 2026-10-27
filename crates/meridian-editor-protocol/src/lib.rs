//! The editor protocol: what a running Meridian game and its editor
//! (Meridian IDLE) say to each other, as typed Substrate Data messages.
//!
//! Tier: above `meridian-scene` (whose component names and
//! `ComponentRegistry::describe()` it reuses) and below `engine-core`
//! (which will host the other end). Pure protocol: no async, no sockets,
//! no I/O beyond a generic [`std::io::Read`]/[`std::io::Write`] through
//! `substrate_data::io`. The editor sends a [`RequestEnvelope`]; the host
//! answers with [`HostMessage`]s, each a response to one request or an
//! [`Event`] it was subscribed to.
//!
//! **Append-only.** Variants are only ever added at the end of
//! [`Request`], [`Response`], [`Event`], [`RequestError`] and the other
//! message enums, fields only at the end of a struct, and nothing is
//! removed or reordered; golden tests pin it. A change that cannot be
//! made that way bumps [`ProtocolVersion`]. Messages, versioning and how
//! the IDE consumes them: docs/editor-protocol.md.

mod ids;
mod message;
mod version;
mod wire;

pub use ids::{ComponentRef, EntityRef, RequestId};
pub use message::{
    Event, EventTopic, HostMessage, LogLevel, PlayState, Request, RequestEnvelope, RequestError,
    Response,
};
pub use version::ProtocolVersion;
pub use wire::{FrameError, read_message, write_message};
