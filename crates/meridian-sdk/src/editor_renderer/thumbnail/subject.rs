//! What a thumbnail shows.

use meridian_rendering_domain::{Material, Scene3d};

/// An asset to preview with [`EditorRenderer::thumbnail`](crate::editor_renderer::EditorRenderer::thumbnail).
#[derive(Debug, Clone)]
pub enum ThumbnailSubject {
    /// The glTF model at this path (`.glb`, or `.gltf` with its files beside it), at rest.
    Model(String),
    /// A material on a sphere.
    Material(Material),
    /// A plain colour (linear RGBA) on a sphere.
    Color([f32; 4]),
    /// A ready-made scene (a prefab or scene already loaded into renderables),
    /// framed by its bounds.
    Scene(Box<Scene3d>),
}
