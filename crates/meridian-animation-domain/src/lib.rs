//! Skeletal animation vocabulary and math (ADR 047).

mod clips;
mod rigs;
mod skeletons;
mod state_machine;

pub use clips::{
    Channel, Clip, ClipError, Crossfade, Interpolation, Keys, Playback, Repeat, Track, WeightKeys,
    WeightsChannel,
};
pub use rigs::{
    Animation, AnimationLayer, AnimationPlayer, AnimationTarget, Animator, MAX_MIX, Mix, MixEntry,
    Rig, RootDelta, RootMotion, SkeletonClip,
};
pub use skeletons::{
    Joint, JointId, JointMatrix, JointTransform, MAX_JOINTS, Pose, Skeleton, SkeletonError,
};
pub use state_machine::{
    AnimationController, AnimationGraph, AnimationState, Blend1d, Blend2d, Condition, GraphError,
    Motion, ParamId, ParamKind, StateId, Transition,
};
