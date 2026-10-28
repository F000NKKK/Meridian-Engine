//! A decoded multi-part model: geometry, materials and still-encoded images.

mod encoded_image;
mod model_asset;
mod model_material;
mod model_part;

pub use encoded_image::EncodedImage;
pub use model_asset::ModelAsset;
pub use model_material::ModelMaterial;
pub use model_part::ModelPart;
