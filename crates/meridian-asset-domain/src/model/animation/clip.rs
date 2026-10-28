//! A piece of animation.

use super::ModelChannel;

/// The channels of one glTF animation that move joints.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelClip {
    /// The channels, which run together.
    pub channels: Vec<ModelChannel>,
}
