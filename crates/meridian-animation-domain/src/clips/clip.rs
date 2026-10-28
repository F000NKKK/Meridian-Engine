//! A set of channels played together.

use super::Channel;
use crate::Pose;

/// A piece of animation: channels that run on one timeline starting at zero.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Clip {
    channels: Vec<Channel>,
    duration: f32,
}

impl Clip {
    /// A clip of `channels`, as long as the latest key of any of them.
    #[must_use]
    pub fn new(channels: Vec<Channel>) -> Self {
        let duration = channels
            .iter()
            .map(|channel| channel.track.end_time())
            .fold(0.0, f32::max);
        Self { channels, duration }
    }

    /// The channels.
    #[must_use]
    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }

    /// The length of the timeline in seconds.
    #[must_use]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Sets every property the clip animates in `pose` to its value at
    /// `time`; properties no channel animates keep what they were. A channel
    /// aimed at a joint the pose does not have is skipped.
    pub fn sample(&self, time: f32, pose: &mut Pose) {
        for channel in &self.channels {
            if let Some(transform) = pose.joint_mut(channel.target) {
                channel.track.apply(time, transform);
            }
        }
    }
}
