//! The component that plays a rig on an entity.

use std::sync::Arc;

use meridian_ecs::Component;

use super::{AnimationPlayer, Rig};

/// Plays a [`Rig`] on the entity it is attached to. The rig is shared, so
/// many entities can play one model at their own pace and with their own
/// crossfades. An animation system advances the player each frame and
/// writes the result to the entities whose
/// [`AnimationTarget`](super::AnimationTarget) names this one.
#[derive(Debug, Clone)]
pub struct Animator {
    /// What to play.
    pub rig: Arc<Rig>,
    /// What is playing now.
    pub player: AnimationPlayer,
}

impl Component for Animator {}
