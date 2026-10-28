//! A model's skeletons and animations, and playing them.

mod animation;
mod animation_target;
mod animator;
mod layer;
mod player;
mod rig;
mod root_motion;
mod skeleton_clip;

pub use animation::Animation;
pub use animation_target::AnimationTarget;
pub use animator::Animator;
pub use layer::AnimationLayer;
pub use player::AnimationPlayer;
pub use rig::Rig;
pub use root_motion::RootMotion;
pub use skeleton_clip::SkeletonClip;
