//! The game side of the editor protocol, without I/O.
//!
//! A [`Session`] is one editor connection's state: whether it has said
//! `Hello`, and which [`meridian_editor_domain::EventTopic`]s it
//! subscribed to. [`Session::handle`] takes a decoded
//! [`meridian_editor_domain::RequestEnvelope`] and the [`Game`] it acts on (the `World`, the
//! `ComponentRegistry` and a [`SimulationControl`]) and returns the
//! [`meridian_editor_domain::HostMessage`]s to send: the response, then
//! the events the request caused that the editor subscribed to.
//!
//! Tier: above `meridian-editor-domain`, `meridian-data-substrate` and `ecs-core`,
//! below whoever serves a stream (`meridian-editor-tcp`) and `engine-core`.
//! Synchronous: nothing here waits on anything outside the process.
//! Lifecycle, what is and is not implemented: docs/editor-protocol.md.

mod game;
mod handlers;
mod incoming;
mod manual_simulation;
mod outcome;
mod request_handler;
mod session;
mod simulation_control;
mod subscriptions;

pub use game::Game;
pub use incoming::{Incoming, unsupported};
pub use manual_simulation::ManualSimulation;
pub use request_handler::RequestHandler;
pub use session::Session;
pub use simulation_control::SimulationControl;
pub use subscriptions::Subscriptions;
