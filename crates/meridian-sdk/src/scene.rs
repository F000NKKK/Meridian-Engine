//! The windowed application's graphics session: the wgpu recorder, the
//! registries and the asset cache an application builds once. The mesh
//! builders it used to hold are `meridian_rendering_domain`'s `shapes`; the
//! per-frame logic is `meridian_rendering_app::RenderSession`.

use meridian_rendering_app::{FrameRecorder, RenderSession};
use meridian_rendering_domain::{MeshHandle, TextureHandle};
use meridian_rendering_wgpu::{Device, GpuFrameRecorder, Surface};

mod environment_loading;

use self::environment_loading::load_environment_map;
use crate::assets::AssetCache;
use crate::model_loading::ModelPiece;

/// What a windowed application builds once: a [`RenderSession`] over the wgpu
/// [`GpuFrameRecorder`] (device, swapchain, renderer, bloom, textures, plus the
/// mesh and material registries to fill) and the asset cache.
///
/// Constructed from an already-open `Device`/`Surface` (the windowed handshake
/// itself stays in the application's own `on_ready`, the only place that needs
/// to name `winit`/`wgpu`-adjacent types at all). Loading goes through the
/// forwarding methods [`load_texture`](Self::load_texture) and
/// [`load_mesh_obj`](Self::load_mesh_obj), whose decode/cache/register logic
/// lives in [`crate::assets`].
#[derive(Debug)]
pub struct GraphicsSession {
    pub render: RenderSession<GpuFrameRecorder>,
    pub assets: AssetCache,
}

impl GraphicsSession {
    /// Builds with the default shadow map resolution — see
    /// [`with_shadow_map_size`](Self::with_shadow_map_size) to pick a specific
    /// one (sharper-but-costlier vs. cheaper-but-blockier shadows).
    #[must_use]
    pub fn new(device: Device, surface: Surface, width: u32, height: u32) -> Self {
        Self::with_shadow_map_size(
            device,
            surface,
            width,
            height,
            meridian_rendering_wgpu::DEFAULT_SHADOW_MAP_SIZE,
        )
    }

    /// Builds with a chosen shadow map resolution.
    #[must_use]
    pub fn with_shadow_map_size(
        device: Device,
        surface: Surface,
        width: u32,
        height: u32,
        shadow_map_size: u32,
    ) -> Self {
        let recorder =
            GpuFrameRecorder::with_shadow_map_size(device, surface, width, height, shadow_map_size);
        Self {
            render: RenderSession::new(recorder),
            assets: AssetCache::new(),
        }
    }

    /// Rebuilds every size-dependent piece after a window resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.render.resize(width, height);
    }

    /// Forwards to [`AssetCache::load_texture`].
    pub fn load_texture(&mut self, path: &str) -> TextureHandle {
        self.assets.load_texture(&mut self.render.recorder, path)
    }

    /// Forwards to [`AssetCache::load_model_gltf`].
    ///
    /// # Errors
    ///
    /// The file's read, decode or registration error, as text.
    pub fn load_model_gltf(&mut self, path: &str) -> Result<Vec<ModelPiece>, String> {
        self.assets
            .load_model_gltf(&mut self.render.meshes, &mut self.render.recorder, path)
    }

    /// Forwards to [`AssetCache::load_mesh_obj`].
    ///
    /// # Errors
    ///
    /// The file's read, decode or mesh-validation error, as text.
    pub fn load_mesh_obj(&mut self, path: &str) -> Result<MeshHandle, String> {
        self.assets.load_mesh_obj(&mut self.render.meshes, path)
    }

    /// Lights the scene with the equirectangular Radiance `.hdr` panorama at
    /// `path` (ADR 036): decoded, prefiltered and handed to the recorder.
    ///
    /// # Errors
    ///
    /// The file's read, decode or validation error, as text.
    pub fn load_environment(&mut self, path: &str) -> Result<(), String> {
        let map = load_environment_map(path)?;
        self.render.recorder.set_environment(Some(&map));
        Ok(())
    }
}
