//! [`Parent`]: the hierarchy link a loaded scene gives an entity.

use meridian_ecs::{Component, Entity};

/// The entity this one hangs under, as a scene's `parent: "@entity:…"` said.
///
/// Lives here for now: `ecs-core` has no hierarchy yet, and the component
/// moves into it once it has one (docs/roadmap.md). Loading inserts it for
/// every entity with a `parent`; saving writes it back as the parent's
/// document key. Nothing propagates transforms along it yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Parent(pub Entity);

impl Component for Parent {}
