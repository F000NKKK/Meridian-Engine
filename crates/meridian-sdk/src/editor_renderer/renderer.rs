//! The renderer itself: one headless device, reused for every frame.

use meridian_ecs::World;
use meridian_gac::{Motor3, Vec3};
use meridian_rendering_app::{FrameOutcome, FrameRecorder as _};
use meridian_rendering_domain::{
    Background, EnvironmentMap, Light, Scene3d, Sky, extract_scene3d, look_at_upright,
};

use super::studio::studio_environment;
use super::{EditorCamera, EditorRendererError};
use crate::GraphicsSession;

const SUN: Vec3 = Vec3 {
    x: -0.4,
    y: -1.0,
    z: -0.3,
};
/// The flat dark backdrop behind a thumbnail, linear.
const STUDIO_BACKDROP: [f64; 4] = [0.05, 0.055, 0.065, 1.0];
const CLEAR: [f64; 4] = [0.18, 0.2, 0.24, 1.0];

/// Renders ECS worlds and scenes to RGBA8 pixels or directly to a native
/// surface. One GPU device and one sky are made once and reused. Meshes and
/// materials the world refers to by handle live in
/// [`session_mut`](Self::session_mut)'s registries.
#[derive(Debug)]
pub struct EditorRenderer {
    pub(super) session: GraphicsSession,
    pub(super) size: (u32, u32),
    pub(super) pixels: Vec<u8>,
    present: bool,
    /// The sky of the viewport, kept to put back after a thumbnail.
    sky: Option<EnvironmentMap>,
    /// The studio of thumbnails, built the first time one is made.
    studio: Option<EnvironmentMap>,
    studio_active: bool,
}

impl EditorRenderer {
    /// A renderer producing `width` x `height` frames. Blocks while the GPU
    /// device opens; do not call it from inside an async runtime (use
    /// [`new_async`](Self::new_async) there).
    ///
    /// # Errors
    ///
    /// When no GPU device can be opened.
    pub fn new(width: u32, height: u32) -> Result<Self, EditorRendererError> {
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        runtime.block_on(Self::new_async(width, height))
    }

    /// [`new`](Self::new) for callers already in an async runtime.
    ///
    /// # Errors
    ///
    /// When no GPU device can be opened.
    pub async fn new_async(width: u32, height: u32) -> Result<Self, EditorRendererError> {
        let session = GraphicsSession::headless(width, height).await?;
        Ok(Self::from_session(session, width, height, false))
    }

    /// A renderer presenting directly to `target`, without reading pixels back.
    ///
    /// # Errors
    ///
    /// When no GPU device or surface can be opened.
    pub fn new_windowed(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<Self, EditorRendererError> {
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        let (device, surface) = runtime.block_on(meridian_rendering_wgpu::Device::new_windowed(
            target, width, height,
        ))?;
        let session = GraphicsSession::new(device, surface, width, height);
        Ok(Self::from_session(session, width, height, true))
    }

    fn from_session(mut session: GraphicsSession, width: u32, height: u32, present: bool) -> Self {
        let sky = Sky {
            sun_direction: [-SUN.x, -SUN.y, -SUN.z],
            sun_radiance: [30.0, 27.0, 22.0],
            sun_radius: 0.06,
            ..Sky::default()
        };
        let sky = EnvironmentMap::from_sky(&sky).ok();
        session.render.recorder.set_environment(sky.as_ref());
        Self {
            session,
            size: (width.max(1), height.max(1)),
            pixels: Vec::new(),
            present,
            sky,
            studio: None,
            studio_active: false,
        }
    }

    /// The session whose registries hold the meshes and materials to draw.
    pub fn session_mut(&mut self) -> &mut GraphicsSession {
        &mut self.session
    }

    /// The frame size, in pixels.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Changes the frame size (a viewport resize).
    pub fn resize(&mut self, width: u32, height: u32) {
        let size = (width.max(1), height.max(1));
        if size != self.size {
            self.session.resize(size.0, size.1);
            self.size = size;
        }
    }

    /// Draws `world` from `camera`: every entity with a transform and a
    /// `MeshRenderer`, lit by its `LightSource`s (a default sun when it has
    /// none). The pixels are tightly packed RGBA8 rows, top first; `None` when
    /// the frame could not be drawn.
    pub fn render(&mut self, world: &World, camera: &EditorCamera) -> Option<&[u8]> {
        let aspect = self.aspect();
        let scene = extract_scene3d(world, camera.camera(aspect));
        self.render_scene(scene)
    }

    /// Draws a ready-made scene with its own camera (sky, a default sun and
    /// ambient light are added where it has none).
    pub fn render_scene(&mut self, mut scene: Scene3d) -> Option<&[u8]> {
        self.use_environment(false);
        if scene.lights.is_empty() {
            scene.lights.push(default_sun());
            scene.ambient_ground = [0.12, 0.1, 0.09];
            scene.ambient_sky = [0.22, 0.25, 0.3];
        }
        scene.background = Background::Environment { blur: 0.0 };
        if self.present {
            let outcome = self.session.render.render(&scene, CLEAR, self.size);
            return (outcome == FrameOutcome::Presented).then_some(&[]);
        }
        self.pixels = self.session.render_to_pixels(&scene, CLEAR)?;
        Some(&self.pixels)
    }

    /// Draws `scene` as a thumbnail: lit and reflected by the studio, which
    /// over a dark backdrop. The scene's own lights are used as given.
    pub(super) fn render_studio(&mut self, mut scene: Scene3d) -> Option<Vec<u8>> {
        self.use_environment(true);
        scene.ambient_ground = [0.0; 3];
        scene.ambient_sky = [0.0; 3];
        if scene.lights.is_empty() {
            scene.lights.push(studio_key());
        }
        scene.background = Background::Clear;
        self.session.render_to_pixels(&scene, STUDIO_BACKDROP)
    }

    /// Makes the studio (`true`) or the viewport sky (`false`) the one the
    /// recorder lights with; nothing happens when it already is.
    fn use_environment(&mut self, studio: bool) {
        if self.studio_active == studio {
            return;
        }
        if studio && self.studio.is_none() {
            self.studio = Some(studio_environment());
        }
        let map = if studio { &self.studio } else { &self.sky };
        self.session.render.recorder.set_environment(map.as_ref());
        self.studio_active = studio;
    }

    #[expect(clippy::cast_precision_loss, reason = "frame sizes are far below 2^24")]
    pub(super) fn aspect(&self) -> f32 {
        self.size.0 as f32 / self.size.1 as f32
    }
}

fn default_sun() -> Light {
    Light::Directional {
        direction: Motor3::from_rotation_translation(look_at_upright(Vec3::ZERO, SUN), Vec3::ZERO),
        color: [1.0, 0.96, 0.9],
        intensity: 1.1,
    }
}

/// A soft key from above left of the camera, so a form reads as round
/// beside what the studio's soft boxes light.
fn studio_key() -> Light {
    let travel = Vec3::new(0.5, -0.7, -0.6);
    Light::Directional {
        direction: Motor3::from_rotation_translation(
            look_at_upright(Vec3::ZERO, travel),
            Vec3::ZERO,
        ),
        color: [1.0, 0.97, 0.92],
        intensity: 0.7,
    }
}
