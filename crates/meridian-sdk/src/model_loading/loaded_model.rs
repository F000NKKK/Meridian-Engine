//! A loaded model: its pieces and its rig.

use std::sync::Arc;

use super::model_piece::ModelPiece;
use super::rig::ModelRig;

/// A loaded model: the pieces to draw and the rig that moves them.
#[derive(Debug, Clone)]
pub struct LoadedModel {
    /// One piece per part, in the file's order.
    pub pieces: Vec<ModelPiece>,
    /// The skeletons and animations of the file's skins; empty for a static model.
    /// Shared, so every instance spawned from the model plays the same rig.
    pub rig: Arc<ModelRig>,
}
