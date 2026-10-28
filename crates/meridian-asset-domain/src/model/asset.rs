//! A decoded model.

use super::{EncodedImage, ModelPart};

/// A model file decoded to CPU data: its drawable parts and the images
/// they reference, not yet uploaded to the GPU.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelAsset {
    /// The drawable pieces, in scene order.
    pub parts: Vec<ModelPart>,
    /// The model's images, indexed like the source file's image list.
    pub images: Vec<EncodedImage>,
}
