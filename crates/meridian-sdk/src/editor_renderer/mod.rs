//! An offscreen renderer for tools: an editor viewport and asset thumbnails.

mod binding;
mod camera;
mod error;
mod framing;
mod overlay;
mod picking;
mod renderer;
mod studio;
mod thumbnail;

pub use binding::{
    AssetBinder, BuiltinAssets, GltfFolder, ModelPieces, TextureSlot, UnboundReference,
    bind_material,
};
pub use camera::EditorCamera;
pub use error::EditorRendererError;
pub use overlay::ViewOptions;
pub use renderer::EditorRenderer;
pub use thumbnail::{THUMBNAIL_RENDERER_VERSION, ThumbnailError, ThumbnailSubject};
