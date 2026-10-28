//! A decoded glTF model registered with the GPU ports: one mesh and one
//! material per part (ADR 030), and the rig that moves them (ADR 047).

mod image_uploads;
mod loaded_model;
mod mixing;
mod model_piece;
mod player;
mod register;
pub(crate) mod rig;
#[cfg(test)]
mod tests;

pub use loaded_model::LoadedModel;
pub use mixing::AnimationLayer;
pub use model_piece::ModelPiece;
pub use player::AnimationPlayer;
pub(crate) use register::register;
