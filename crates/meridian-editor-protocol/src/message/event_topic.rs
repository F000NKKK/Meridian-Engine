//! [`EventTopic`]: a group of events one can subscribe to.

use substrate_data::Schema;

/// A group of [`crate::Event`]s one subscribes to. Variants are only ever
/// appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Schema)]
pub enum EventTopic {
    /// `EntityAdded` and `EntityRemoved`.
    Entities,
    /// `ComponentChanged`.
    Components,
    /// `Log`.
    Log,
    /// `PlayStateChanged`.
    PlayState,
}
