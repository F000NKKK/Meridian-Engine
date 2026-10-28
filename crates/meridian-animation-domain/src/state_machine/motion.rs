//! What a state plays.

use super::{Blend1d, Blend2d, ParamId};
use crate::Mix;

/// What a state plays: one animation, or a blend that the graph's parameters
/// steer.
#[derive(Debug, Clone, PartialEq)]
pub enum Motion {
    /// One animation (an index into the rig's animations); none is the rest
    /// pose.
    Clip(Option<usize>),
    /// A blend along one parameter.
    Blend1d(Blend1d),
    /// A blend across two parameters.
    Blend2d(Blend2d),
}

impl Motion {
    /// The parameters the motion reads.
    pub(super) fn params(&self) -> Vec<ParamId> {
        match self {
            Self::Clip(_) => Vec::new(),
            Self::Blend1d(blend) => vec![blend.param],
            Self::Blend2d(blend) => vec![blend.x, blend.y],
        }
    }

    /// The mix of animations for the parameter values `float` reads.
    pub(super) fn mix(&self, float: impl Fn(ParamId) -> f32) -> Mix {
        match self {
            Self::Clip(animation) => Mix::single(*animation),
            Self::Blend1d(blend) => Mix::weighted(&blend.weights(float(blend.param))),
            Self::Blend2d(blend) => Mix::weighted(&blend.weights(float(blend.x), float(blend.y))),
        }
    }

    /// Whether the mix changes as parameters move.
    pub(super) fn is_blend(&self) -> bool {
        !matches!(self, Self::Clip(_))
    }
}
