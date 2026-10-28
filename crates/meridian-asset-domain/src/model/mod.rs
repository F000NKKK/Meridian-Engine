//! A decoded multi-part model: geometry, materials and still-encoded images.

mod animation;
mod asset;
mod encoded_image;
mod material;
mod part;
mod skin;

pub use animation::{ModelChannel, ModelClip, ModelInterpolation, ModelProperty};
pub use asset::ModelAsset;
pub use encoded_image::EncodedImage;
pub use material::ModelMaterial;
pub use part::ModelPart;
pub use skin::{ModelJoint, ModelSkeleton, PartSkin};
