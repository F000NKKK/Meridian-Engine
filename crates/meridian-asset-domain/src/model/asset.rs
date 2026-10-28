//! A decoded model.

use super::{EncodedImage, ModelClip, ModelPart, ModelSkeleton};

/// A model file decoded to CPU data: its drawable parts and the images
/// they reference, not yet uploaded to the GPU.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelAsset {
    /// The drawable pieces, in scene order.
    pub parts: Vec<ModelPart>,
    /// The model's images, indexed like the source file's image list.
    pub images: Vec<EncodedImage>,
    /// The skeletons of the file's skins, indexed like its skin list.
    pub skeletons: Vec<ModelSkeleton>,
    /// The animations that move joints, in file order.
    pub clips: Vec<ModelClip>,
}
