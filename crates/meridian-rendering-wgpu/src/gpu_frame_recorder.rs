//! The `wgpu` implementation of [`FrameRecorder`].

use meridian_asset_domain::ImageAsset;
use meridian_rendering_app::{FrameOutcome, FrameRecorder, FrameRequest, TextureStore};
use meridian_rendering_domain::{EnvironmentMap, TextureHandle, TonemapConfig};

use crate::overlay_pass::OverlayPass;
use crate::{
    BloomInput, BloomPass, ColorFormat, CommandBuffer, DEFAULT_SHADOW_MAP_SIZE, DepthTexture,
    Device, GpuTextureStore, PassProfiler, PassTiming, PreparedScene, SceneAssets, SceneRenderer,
    SkyPass, Surface, SurfaceFrame, Texture, TonemapPass, submit_scene3d,
};

/// The format of the scene target the 3D passes draw into (ADR 034).
const SCENE_FORMAT: ColorFormat = ColorFormat::HdrRgba16F;

/// Everything one windowed view draws with: the device, the swapchain, the HDR
/// scene target and its depth target, the scene renderer, the bloom and tone
/// mapping passes and the GPU textures.
///
/// Built once from an already-open [`Device`] and [`Surface`] (the windowed
/// handshake stays in the application's own `on_ready`). It is the
/// [`FrameRecorder`] of the rendering context and, through its textures, a
/// [`TextureStore`].
#[derive(Debug)]
pub struct GpuFrameRecorder {
    device: Device,
    surface: Surface,
    scene: Texture,
    depth: DepthTexture,
    renderer: SceneRenderer,
    bloom: BloomPass,
    sky: SkyPass,
    tonemap: TonemapPass,
    overlays: OverlayPass,
    textures: GpuTextureStore,
    size: [u32; 2],
    profiler: Option<PassProfiler>,
    pass_timings: Vec<PassTiming>,
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
        let scene = device.create_offscreen_color_texture(width, height, SCENE_FORMAT);
        let renderer = SceneRenderer::with_shadow_map_size(&device, SCENE_FORMAT, shadow_map_size);
        let bloom = BloomPass::new(&device, width, height);
        let sky = SkyPass::new(&device, SCENE_FORMAT);
        let tonemap = TonemapPass::new(&device, &surface);
        let overlays = OverlayPass::new(&device, &surface);
        let textures = GpuTextureStore::new(device.clone());
        Self {
            device,
            surface,
            scene,
            depth,
            renderer,
            bloom,
            sky,
            tonemap,
            overlays,
            textures,
            size: [width, height],
            profiler: None,
            pass_timings: Vec::new(),
        }
    }
}

impl GpuFrameRecorder {
    /// Draws the 3D scene (its backdrop first) into the HDR target, blooms it there and tone maps
    /// the result into `frame`.
    fn draw_scene(
        &self,
        commands: &mut CommandBuffer<'_>,
        request: &FrameRequest<'_>,
        prepared: &PreparedScene,
        viewports: &[PreparedScene],
        frame: &SurfaceFrame,
    ) {
        {
            commands.label_next_pass("scene (sky + meshes)");
            let mut pass = commands.begin_render_pass(
                self.scene.view(),
                request.clear_color,
                Some(&self.depth),
            );
            self.sky.draw(
                &self.device,
                &mut pass,
                request.scene,
                &self.renderer.environment,
            );
            submit_scene3d(&self.renderer, &mut pass, prepared);
        }
        for (viewport, prepared) in request.viewports.iter().zip(viewports) {
            let [x, y, width, height] = viewport.rect;
            let mut pass = commands.begin_viewport_pass(self.scene.view(), &self.depth);
            pass.set_viewport(x, y, width, height);
            pass.set_scissor_rect(x, y, width, height);
            submit_scene3d(&self.renderer, &mut pass, prepared);
        }
        self.bloom.apply(
            &self.device,
            commands,
            &BloomInput {
                renderer: &self.renderer,
                prepared,
                textures: self.textures.registry(),
                target: &self.scene,
            },
        );
        self.tonemap
            .apply(&self.device, commands, &self.scene, frame);
    }

    /// The last frame as RGBA8, when this recorder draws into an offscreen
    /// [`Surface`] (see [`Surface::offscreen`]); `None` for a window.
    #[must_use]
    pub fn read_pixels(&self) -> Option<Vec<u8>> {
        self.surface.read_pixels(&self.device)
    }

    /// The width and height of the frames it draws.
    #[must_use]
    pub fn frame_size(&self) -> (u32, u32) {
        (self.size[0], self.size[1])
    }

    /// The name the GPU adapter reports (e.g. `llvmpipe` for a software one).
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        self.device.adapter_name()
    }

    /// Starts timing every render pass on the GPU, when the device offers
    /// timestamp queries; returns whether it does. Each frame then waits for
    /// the GPU to read its times back, so leave it off outside measuring.
    pub fn enable_profiling(&mut self) -> bool {
        self.profiler = PassProfiler::new(&self.device);
        self.profiler.is_some()
    }

    /// GPU time per named pass of the last frame (empty unless
    /// [`enable_profiling`](Self::enable_profiling) succeeded).
    #[must_use]
    pub fn pass_timings(&self) -> &[PassTiming] {
        &self.pass_timings
    }

    /// Replaces the exposure and tone curve applied to the next frames.
    pub fn set_tonemap(&mut self, config: TonemapConfig) {
        self.tonemap.config = config;
    }
}

impl FrameRecorder for GpuFrameRecorder {
    /// Records the shadow-casting depth pass (its own pass, before the main
    /// colour pass opens), acquires the swapchain frame, draws the scene, applies
    /// bloom, draws the overlays unblurred over it and presents. A lost or outdated swapchain skips the frame, logs
    /// it, and reports [`FrameOutcome::TargetUnavailable`] instead of leaving a
    /// gray window.
    fn record_frame(&mut self, request: &FrameRequest<'_>) -> FrameOutcome {
        let device = self.device.clone();
        let mut commands = device
            .create_command_buffer()
            .with_profiler(self.profiler.as_ref());
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
        let viewports: Vec<_> = request
            .viewports
            .iter()
            .map(|viewport| {
                self.renderer.prepare_viewport(
                    &self.device,
                    &mut commands,
                    &viewport.scene,
                    &SceneAssets {
                        meshes: request.meshes,
                        materials: request.materials,
                        textures: self.textures.registry(),
                    },
                )
            })
            .collect();

        let overlays = self.overlays.prepare(
            &self.device,
            self.textures.registry(),
            request.overlays,
            self.size,
        );

        let frame = match self.surface.acquire_frame() {
            Ok(frame) => frame,
            Err(err) => {
                meridian_diagnostics::log_warn!("swapchain frame unavailable ({err})");
                return FrameOutcome::TargetUnavailable;
            }
        };

        self.draw_scene(&mut commands, request, &prepared, &viewports, &frame);
        self.overlays.draw(&mut commands, &frame, &overlays);
        commands.submit();
        if let Some(profiler) = &self.profiler {
            self.pass_timings = profiler.collect(&self.device);
        }
        frame.present(&self.device);
        FrameOutcome::Presented
    }

    /// Rebuilds every size-dependent piece (swapchain, depth buffer, bloom's
    /// offscreen targets); the scene renderer's pipelines name no resolution and
    /// are kept.
    fn set_environment(&mut self, environment: Option<&EnvironmentMap>) {
        self.renderer.set_environment(&self.device, environment);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(&self.device, width, height);
        self.depth = self.device.create_depth_texture(width, height);
        self.scene = self
            .device
            .create_offscreen_color_texture(width, height, SCENE_FORMAT);
        self.bloom = BloomPass::new(&self.device, width, height);
        self.size = [width, height];
    }
}

impl TextureStore for GpuFrameRecorder {
    fn upload(&mut self, image: &ImageAsset) -> TextureHandle {
        self.textures.upload(image)
    }

    fn upload_linear(&mut self, image: &ImageAsset) -> TextureHandle {
        self.textures.upload_linear(image)
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
