//! A piece of animation.

use super::{ModelChannel, ModelWeightsChannel};

/// The channels of one glTF animation that move joints or blend shapes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelClip {
    /// The channels, which run together.
    pub channels: Vec<ModelChannel>,
    /// The morph weight channels, which run together with them.
    pub weights: Vec<ModelWeightsChannel>,
}
