//! A decoded multi-part model: geometry, materials and still-encoded images.

mod animation;
mod asset;
mod encoded_image;
mod material;
mod morph;
mod part;
mod skin;

pub use animation::{
    ModelChannel, ModelClip, ModelInterpolation, ModelProperty, ModelWeightsChannel,
};
pub use asset::ModelAsset;
pub use encoded_image::EncodedImage;
pub use material::ModelMaterial;
pub use morph::{MorphTarget, PartMorph};
pub use part::ModelPart;
pub use skin::{ModelJoint, ModelSkeleton, PartSkin};
