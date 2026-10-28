//! The renderer itself: one headless device, reused for every frame.

use meridian_ecs::World;
use meridian_gac::{Motor3, Vec3};
use meridian_rendering_domain::{
    Background, EnvironmentMap, Light, Scene3d, Sky, extract_scene3d, look_at_upright,
};

use super::{EditorCamera, EditorRendererError};
use crate::GraphicsSession;
use meridian_rendering_app::FrameRecorder as _;

const SUN: Vec3 = Vec3 {
    x: -0.4,
    y: -1.0,
    z: -0.3,
};
const CLEAR: [f64; 4] = [0.18, 0.2, 0.24, 1.0];

/// Renders ECS worlds and scenes to RGBA8 pixels with no window: an editor
/// viewport and asset thumbnails. One GPU device and one sky are made once and
/// reused. Meshes and materials the world refers to by handle live in
/// [`session_mut`](Self::session_mut)'s registries.
#[derive(Debug)]
pub struct EditorRenderer {
    pub(super) session: GraphicsSession,
    pub(super) size: (u32, u32),
    pub(super) pixels: Vec<u8>,
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
        let mut session = GraphicsSession::headless(width, height).await?;
        let sky = Sky {
            sun_direction: [-SUN.x, -SUN.y, -SUN.z],
            sun_radiance: [30.0, 27.0, 22.0],
            sun_radius: 0.06,
            ..Sky::default()
        };
        if let Ok(map) = EnvironmentMap::from_sky(&sky) {
            session.render.recorder.set_environment(Some(&map));
        }
        Ok(Self {
            session,
            size: (width.max(1), height.max(1)),
            pixels: Vec::new(),
        })
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
        if scene.lights.is_empty() {
            scene.lights.push(default_sun());
            scene.ambient_ground = [0.12, 0.1, 0.09];
            scene.ambient_sky = [0.22, 0.25, 0.3];
        }
        scene.background = Background::Environment { blur: 0.0 };
        self.pixels = self.session.render_to_pixels(&scene, CLEAR)?;
        Some(&self.pixels)
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
