//! Binding the asset references a file holds to the handles the renderer
//! draws with.

mod asset_binder;
mod builtin_assets;
mod material_binding;

pub use asset_binder::{AssetBinder, TextureSlot, UnboundReference};
pub use builtin_assets::BuiltinAssets;
pub use material_binding::bind_material;
