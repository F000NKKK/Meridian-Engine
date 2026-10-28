//! [`LoadedSave`]: a save read and its scene spawned.

use super::SaveHeader;
use crate::SceneMap;

/// A save read: its header, the scene it spawned (a [`SceneMap`] from
/// document keys to the fresh entities of this run) and the game's state.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedSave<T> {
    /// What the save says of itself.
    pub header: SaveHeader,
    /// The scene, spawned into the world.
    pub scene: SceneMap,
    /// The game's state, as the game's schema reads it.
    pub state: T,
}
