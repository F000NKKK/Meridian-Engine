//! A decoded multi-part model: geometry, materials and still-encoded images.

mod asset;
mod encoded_image;
mod material;
mod part;

pub use asset::ModelAsset;
pub use encoded_image::EncodedImage;
pub use material::ModelMaterial;
pub use part::ModelPart;
