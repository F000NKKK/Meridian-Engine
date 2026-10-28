//! An animation of a whole model.

use super::SkeletonClip;
use crate::WeightsChannel;

/// A glTF animation: the clips of the skeletons it moves and the morph
/// weights it drives, which play together.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Animation {
    /// One clip per skeleton the animation moves.
    pub clips: Vec<SkeletonClip>,
    /// What it does to the morph weights of the model's parts.
    pub weights: Vec<WeightsChannel>,
}

impl Animation {
    /// How long the animation runs: its longest clip or weight track.
    #[must_use]
    pub fn duration(&self) -> f32 {
        let clips = self.clips.iter().map(|entry| entry.clip.duration());
        let weights = self.weights.iter().map(|channel| channel.keys.end_time());
        clips.chain(weights).fold(0.0, f32::max)
    }
}
