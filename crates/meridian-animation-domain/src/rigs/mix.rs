//! Which animations a player is playing at once, and how much of each.

use super::{Animation, Rig};

/// The most animations one mix holds; a blend keeps the heaviest.
pub const MAX_MIX: usize = 4;

/// One animation in a [`Mix`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixEntry {
    /// An index into the rig's animations; none is the rest pose.
    pub animation: Option<usize>,
    /// Its share of the mix; the shares of a mix add up to one.
    pub weight: f32,
    /// Seconds into the animation.
    pub time: f32,
}

/// What a [`player`](super::AnimationPlayer) is playing: one animation, or
/// several at once as a blend tree chose them, kept in step by a shared
/// phase so a slow walk and a fast run put their feet down together.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mix {
    entries: [MixEntry; MAX_MIX],
    len: usize,
}

const EMPTY: MixEntry = MixEntry {
    animation: None,
    weight: 0.0,
    time: 0.0,
};

impl Mix {
    /// One animation (none is the rest pose) at full weight.
    #[must_use]
    pub fn single(animation: Option<usize>) -> Self {
        let mut entries = [EMPTY; MAX_MIX];
        entries[0] = MixEntry {
            animation,
            weight: 1.0,
            time: 0.0,
        };
        Self { entries, len: 1 }
    }

    /// Animations with weights, as a blend tree produces them: the
    /// [`MAX_MIX`] heaviest with a positive weight, scaled to add up to one.
    /// Nothing left is the rest pose.
    #[must_use]
    pub fn weighted(parts: &[(usize, f32)]) -> Self {
        let mut sorted: Vec<(usize, f32)> = parts
            .iter()
            .copied()
            .filter(|(_, weight)| *weight > 0.0)
            .collect();
        sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
        sorted.truncate(MAX_MIX);
        let total: f32 = sorted.iter().map(|(_, weight)| weight).sum();
        if total <= 0.0 {
            return Self::single(None);
        }
        let mut entries = [EMPTY; MAX_MIX];
        for (slot, (animation, weight)) in entries.iter_mut().zip(&sorted) {
            *slot = MixEntry {
                animation: Some(*animation),
                weight: weight / total,
                time: 0.0,
            };
        }
        Self {
            entries,
            len: sorted.len(),
        }
    }

    /// The animations taking part.
    #[must_use]
    pub fn entries(&self) -> &[MixEntry] {
        &self.entries[..self.len]
    }

    /// The animation with the largest share.
    #[must_use]
    pub fn dominant(&self) -> Option<usize> {
        self.entries()
            .iter()
            .max_by(|a, b| a.weight.total_cmp(&b.weight))
            .and_then(|entry| entry.animation)
    }

    /// How long one pass takes: the lengths of the animations averaged by
    /// their shares.
    #[must_use]
    pub fn duration(&self, rig: &Rig) -> f32 {
        self.entries()
            .iter()
            .map(|entry| entry.weight * length(rig, entry.animation))
            .sum()
    }

    /// Puts every animation at `phase` (zero to one) of its own length.
    pub(super) fn seek(&mut self, rig: &Rig, phase: f32) {
        for entry in &mut self.entries[..self.len] {
            entry.time = phase * length(rig, entry.animation);
        }
    }
}

/// The length of `animation` in seconds; the rest pose has none.
pub(super) fn length(rig: &Rig, animation: Option<usize>) -> f32 {
    animation
        .and_then(|index| rig.animations.get(index))
        .map_or(0.0, Animation::duration)
}
