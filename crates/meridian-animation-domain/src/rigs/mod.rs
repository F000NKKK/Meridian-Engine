//! A model's skeletons and animations, and playing them.

mod animation;
mod animation_target;
mod animator;
mod layer;
mod mix;
mod player;
mod rig;
mod root_delta;
mod root_motion;
mod running;
mod skeleton_clip;
mod yaw;

pub use animation::Animation;
pub use animation_target::AnimationTarget;
pub use animator::Animator;
pub use layer::AnimationLayer;
pub use mix::{MAX_MIX, Mix, MixEntry};
pub use player::AnimationPlayer;
pub use rig::Rig;
pub use root_delta::RootDelta;
pub use root_motion::RootMotion;
pub use skeleton_clip::SkeletonClip;
