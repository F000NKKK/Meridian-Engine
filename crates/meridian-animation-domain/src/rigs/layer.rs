//! One animation taking part in a mix.

/// One animation taking part in a mix: which one, how far in, and how much
/// of it shows over what came before.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationLayer {
    /// An index into [`Rig::animations`](super::Rig::animations); none is the
    /// rest pose.
    pub animation: Option<usize>,
    /// Seconds into the animation.
    pub time: f32,
    /// Zero shows the layers before it, one shows only this one. The first
    /// layer of a mix has nothing before it, so its weight does not matter.
    pub weight: f32,
}
