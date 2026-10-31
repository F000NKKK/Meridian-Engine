//! Binding the asset references a file holds to the handles the renderer
//! draws with.

mod asset_binder;
mod builtin_assets;
mod gltf_folder;
mod material_binding;

pub use asset_binder::{AssetBinder, ModelPieces, TextureSlot, UnboundReference, bind_references};
pub use builtin_assets::BuiltinAssets;
pub use gltf_folder::GltfFolder;
pub use material_binding::bind_material;
