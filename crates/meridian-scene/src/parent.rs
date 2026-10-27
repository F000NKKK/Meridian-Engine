//! [`Parent`]: the hierarchy link a loaded scene gives an entity.

use meridian_ecs_core::{Component, Entity};

/// The entity this one hangs under, as a scene's `parent: "@entity:…"` said.
///
/// Lives here **for now**: `ecs-core` has no hierarchy concept yet, and
/// team/core's decision is that a parent component moves into `ecs-core`
/// once it has one (docs/roadmap.md). `load_scene` inserts it for every
/// entity with a `parent`; `save_scene` writes it back as the parent's
/// document id. Nothing propagates transforms along it yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Parent(pub Entity);

impl Component for Parent {}
