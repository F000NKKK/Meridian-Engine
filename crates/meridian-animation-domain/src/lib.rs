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
    Animation, AnimationLayer, AnimationPlayer, AnimationTarget, Animator, Rig, RootMotion,
    SkeletonClip,
};
pub use skeletons::{
    Joint, JointId, JointMatrix, JointTransform, MAX_JOINTS, Pose, Skeleton, SkeletonError,
};
pub use state_machine::{
    AnimationController, AnimationGraph, AnimationState, Condition, GraphError, ParamId, ParamKind,
    StateId, Transition,
};
