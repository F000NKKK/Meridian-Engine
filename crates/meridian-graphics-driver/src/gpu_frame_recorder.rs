//! The `wgpu` implementation of [`FrameRecorder`].

use meridian_asset_core::ImageAsset;
use meridian_graphics_core::{
    FrameOutcome, FrameRecorder, FrameRequest, TextureHandle, TextureStore,
};

use crate::{
    BloomInput, BloomPass, DEFAULT_SHADOW_MAP_SIZE, DepthTexture, Device, GpuTextureStore,
    SceneAssets, SceneRenderer, Surface, submit_scene3d,
};

/// Everything one windowed view draws with: the device, the swapchain and its
/// depth target, the scene renderer, the bloom pass and the GPU textures.
///
/// Built once from an already-open [`Device`] and [`Surface`] (the windowed
/// handshake stays in the application's own `on_ready`). It is the
/// [`FrameRecorder`] of the rendering context and, through its textures, a
/// [`TextureStore`].
#[derive(Debug)]
pub struct GpuFrameRecorder {
    device: Device,
    surface: Surface,
    depth: DepthTexture,
    renderer: SceneRenderer,
    bloom: BloomPass,
    textures: GpuTextureStore,
}

impl GpuFrameRecorder {
    /// A recorder with the default shadow map resolution
    /// ([`DEFAULT_SHADOW_MAP_SIZE`]).
    #[must_use]
    pub fn new(device: Device, surface: Surface, width: u32, height: u32) -> Self {
        Self::with_shadow_map_size(device, surface, width, height, DEFAULT_SHADOW_MAP_SIZE)
    }

    /// A recorder with a chosen shadow map resolution: larger is sharper and
    /// costs more per frame, smaller is cheaper and blockier.
    #[must_use]
    pub fn with_shadow_map_size(
        device: Device,
        surface: Surface,
        width: u32,
        height: u32,
        shadow_map_size: u32,
    ) -> Self {
        let depth = device.create_depth_texture(width, height);
        let renderer = SceneRenderer::with_shadow_map_size(&device, &surface, shadow_map_size);
        let bloom = BloomPass::new(&device, width, height, &surface);
        let textures = GpuTextureStore::new(device.clone());
        Self {
            device,
            surface,
            depth,
            renderer,
            bloom,
            textures,
        }
    }
}

impl FrameRecorder for GpuFrameRecorder {
    /// Records the shadow-casting depth pass (its own pass, before the main
    /// colour pass opens), acquires the swapchain frame, draws the scene, applies
    /// bloom and presents. A lost or outdated swapchain skips the frame, logs
    /// it, and reports [`FrameOutcome::TargetUnavailable`] instead of leaving a
    /// gray window.
    fn record_frame(&mut self, request: &FrameRequest<'_>) -> FrameOutcome {
        let mut commands = self.device.create_command_buffer();
        let prepared = self.renderer.render_shadow_pass(
            &self.device,
            &mut commands,
            request.scene,
            &SceneAssets {
                meshes: request.meshes,
                materials: request.materials,
                textures: self.textures.registry(),
            },
        );

        let frame = match self.surface.acquire_frame() {
            Ok(frame) => frame,
            Err(err) => {
                meridian_foundation::log_warn!("swapchain frame unavailable ({err})");
                return FrameOutcome::TargetUnavailable;
            }
        };

        {
            let mut pass =
                commands.begin_render_pass(frame.view(), request.clear_color, Some(&self.depth));
            submit_scene3d(
                &self.device,
                &self.renderer,
                &mut pass,
                request.scene,
                &prepared,
            );
        }
        self.bloom.apply(
            &self.device,
            &mut commands,
            &BloomInput {
                renderer: &self.renderer,
                draw_buffers: &prepared.buffers,
                target_frame: &frame,
            },
        );
        commands.submit();
        frame.present(&self.device);
        FrameOutcome::Presented
    }

    /// Rebuilds every size-dependent piece (swapchain, depth buffer, bloom's
    /// offscreen targets); the scene renderer's pipelines name no resolution and
    /// are kept.
    fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(&self.device, width, height);
        self.depth = self.device.create_depth_texture(width, height);
        self.bloom = BloomPass::new(&self.device, width, height, &self.surface);
    }
}

impl TextureStore for GpuFrameRecorder {
    fn upload(&mut self, image: &ImageAsset) -> TextureHandle {
        self.textures.upload(image)
    }

    fn upload_solid(&mut self, rgba: [u8; 4]) -> TextureHandle {
        self.textures.upload_solid(rgba)
    }

    fn contains(&self, handle: TextureHandle) -> bool {
        self.textures.contains(handle)
    }

    fn release(&mut self, handle: TextureHandle) -> bool {
        self.textures.release(handle)
    }
}
