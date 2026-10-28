//! Previews of single assets: a material on a sphere, a mesh, a model, a scene.

use meridian_gac::Motor3;
use meridian_rendering_domain::{
    Material, MaterialHandle, MeshHandle, Renderable3d, Scene3d, icosphere_mesh_source,
};

use super::framing::bounding_sphere;
use super::{EditorCamera, EditorRenderer, ThumbnailError, ThumbnailSubject};

/// Bumped whenever a thumbnail would look different for the same asset, so a
/// cache keyed with it drops its old images.
pub const THUMBNAIL_RENDERER_VERSION: u32 = 1;

impl EditorRenderer {
    /// A `size` x `size` RGBA8 preview of `subject` under the neutral sky, from
    /// a fixed angle backed off to fit, so equal assets give equal images.
    ///
    /// # Errors
    ///
    /// [`ThumbnailError`] when a model cannot be loaded, the subject draws
    /// nothing, or the frame cannot be drawn.
    pub fn thumbnail(
        &mut self,
        subject: ThumbnailSubject,
        size: u32,
    ) -> Result<Vec<u8>, ThumbnailError> {
        let pixels = match subject {
            ThumbnailSubject::Model(path) => self
                .thumbnail_model(&path, size)
                .map_err(ThumbnailError::Load)?,
            ThumbnailSubject::Material(material) => self.thumbnail_material(material, size),
            ThumbnailSubject::Color(color) => self.thumbnail_material(
                Material {
                    base_color_factor: color,
                    ..Material::default()
                },
                size,
            ),
            ThumbnailSubject::Scene(scene) => self.thumbnail_scene(*scene, size),
        };
        pixels.ok_or(ThumbnailError::Empty)
    }

    /// `material` on a sphere under the neutral sky, `size` x `size` RGBA8.
    pub fn thumbnail_material(&mut self, material: Material, size: u32) -> Option<Vec<u8>> {
        let mesh = self
            .session
            .render
            .meshes
            .register(icosphere_mesh_source(4, 1.0))
            .ok()?;
        let material = self.session.render.materials.register(material);
        self.thumbnail_mesh(mesh, material, size)
    }

    /// One mesh with one material, framed.
    pub fn thumbnail_mesh(
        &mut self,
        mesh: MeshHandle,
        material: MaterialHandle,
        size: u32,
    ) -> Option<Vec<u8>> {
        let scene = Scene3d {
            renderables: vec![still(mesh, material)],
            ..Scene3d::default()
        };
        self.thumbnail_scene(scene, size)
    }

    /// The glTF model at `path`, every part drawn at rest, framed.
    ///
    /// # Errors
    ///
    /// The loader's message when the file cannot be read or decoded.
    pub fn thumbnail_model(&mut self, path: &str, size: u32) -> Result<Option<Vec<u8>>, String> {
        let model = self.session.load_model_gltf(path)?;
        let renderables = model
            .pieces
            .iter()
            .map(|piece| {
                let material = self.session.render.materials.register(piece.material);
                still(piece.mesh, material)
            })
            .collect();
        let scene = Scene3d {
            renderables,
            ..Scene3d::default()
        };
        Ok(self.thumbnail_scene(scene, size))
    }

    /// `scene`'s renderables seen from the default angle, backed off to fit
    /// (its own camera is replaced); `None` when it draws nothing.
    pub fn thumbnail_scene(&mut self, mut scene: Scene3d, size: u32) -> Option<Vec<u8>> {
        let (center, radius) = bounding_sphere(&scene, &self.session.render.meshes)?;
        let before = self.size;
        self.resize(size, size);
        scene.camera = EditorCamera::framing(center, radius).camera(1.0);
        let pixels = self.render_scene(scene).map(<[u8]>::to_vec);
        self.resize(before.0, before.1);
        pixels
    }
}

fn still(mesh: MeshHandle, material: MaterialHandle) -> Renderable3d {
    Renderable3d {
        mesh,
        material,
        frame: Motor3::identity(),
        billboard: false,
        casts_shadow: true,
        palette: None,
        morph_weights: Vec::new(),
    }
}
