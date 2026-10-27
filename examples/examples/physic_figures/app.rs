//! The application handler: window, camera and the frame loop.

use crate::PhysicsRig;
use crate::RenderEntity;
use crate::SUN_DIRECTION;
use crate::SceneEntity;
use crate::load_scene;
use crate::mesh_render_frame;
use crate::mesh_source_for;
use crate::sun_renderable;
use meridian_examples::paths::asset_path;
use meridian_sdk::{
    AppHandler, Camera, Device, FlyCamera, GraphicsSession, InputState, KeyCode, Light, Material,
    Motor3, RenderStage, Renderable3d, RigidBody, Scene3d, StageContext, StageId, Vec3, Window,
    look_at_rotor,
};
use std::sync::{Arc, Mutex};

pub(crate) struct App {
    camera: FlyCamera,
    cursor_grabbed: bool,
    last_frame: std::time::Instant,
    scene_entities: Vec<SceneEntity>,
    physics: PhysicsRig,
    tokio_runtime: tokio::runtime::Runtime,
    /// The render stage's closure reads the current camera from here —
    /// `on_redraw` writes it just before ticking the render stage. This
    /// is the same category of "app state a `Stage` needs but
    /// `RuntimeState` doesn't own" as `magic_figures`' orbit positions
    /// (see `meridian_engine_core`'s own module doc): a camera pose
    /// driven by per-frame input isn't physics/audio/world state.
    render_camera: Arc<Mutex<Camera>>,
    /// `None` until [`AppHandler::on_ready`] registers the render stage
    /// (it needs a real `Device`/`Surface`, built there).
    render_stage: Option<StageId>,
}

impl App {
    pub(crate) fn new() -> Self {
        let tokio_runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        let scene_entities = load_scene();
        let physics = PhysicsRig::new(&scene_entities);
        Self {
            camera: FlyCamera::new(Vec3::new(0.0, 3.0, 9.0)),
            cursor_grabbed: true,
            last_frame: std::time::Instant::now(),
            scene_entities,
            physics,
            tokio_runtime,
            render_camera: Arc::new(Mutex::new(Camera::default())),
            render_stage: None,
        }
    }
}

/// Registers `entity`'s texture, material and mesh.
fn render_entity(base: &mut GraphicsSession, entity: &SceneEntity) -> RenderEntity {
    let texture = base.load_texture(&asset_path(&entity.texture));
    let material = base.materials.register(Material {
        albedo: Some(texture),
        base_color_factor: [1.0, 1.0, 1.0, 1.0],
        ..Default::default()
    });
    let mesh = base
        .meshes
        .register(mesh_source_for(entity))
        .unwrap_or_else(|e| panic!("{} mesh must be valid: {e}", entity.mesh_shape));
    RenderEntity {
        mesh_shape: entity.mesh_shape.clone(),
        collider: entity.collider,
        mesh,
        material,
    }
}

/// The scene's only light: the sun.
fn sun_light() -> Light {
    Light::Directional {
        direction: Motor3::from_rotation_translation(
            look_at_rotor(Vec3::ZERO, SUN_DIRECTION),
            Vec3::ZERO,
        ),
        color: [1.0, 0.96, 0.9],
        intensity: 1.1,
    }
}

/// One renderable per entity, placed at its body's current frame.
fn entity_renderables(entities: &[RenderEntity], bodies: &[RigidBody]) -> Vec<Renderable3d> {
    entities
        .iter()
        .zip(bodies.iter())
        .map(|(re, body)| Renderable3d {
            mesh: re.mesh,
            material: re.material,
            frame: mesh_render_frame(&re.mesh_shape, re.collider, body.frame),
            billboard: false,
            casts_shadow: true,
        })
        .collect()
}

impl AppHandler for App {
    fn on_ready(&mut self, window: &Window) {
        window.set_cursor_grabbed(true);
        let target = window.surface_target();
        let (width, height) = (window.width(), window.height());
        let (device, surface) = self
            .tokio_runtime
            .block_on(Device::new_windowed(target, width, height))
            .expect("failed to create windowed GPU device");
        let mut base = GraphicsSession::new(device, surface, width, height);

        let render_entities: Vec<RenderEntity> = self
            .scene_entities
            .iter()
            .map(|entity| render_entity(&mut base, entity))
            .collect();
        let sun = sun_renderable(&mut base);
        let lights = vec![sun_light()];

        let render_camera = self.render_camera.clone();
        let render_stage = RenderStage::new(
            base,
            window.clone(),
            [0.05, 0.05, 0.08, 1.0],
            move |ctx: &StageContext| {
                let bodies = &ctx.physics().bodies;
                let mut renderables = entity_renderables(&render_entities, bodies);
                renderables.push(sun.clone());
                Scene3d {
                    renderables,
                    lights: lights.clone(),
                    camera: *render_camera
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner),
                    ambient_ground: [0.02, 0.02, 0.025],
                    ambient_sky: [0.04, 0.04, 0.055],
                }
            },
        );
        self.render_stage = Some(self.physics.runtime.add_stage(
            "render",
            &[self.physics.physics_stage],
            render_stage,
        ));
    }

    fn on_redraw(&mut self, window: &Window, input: &InputState) {
        let Some(render_stage) = self.render_stage else {
            return;
        };

        if input.was_key_pressed(KeyCode::Escape) {
            self.cursor_grabbed = !self.cursor_grabbed;
            window.set_cursor_grabbed(self.cursor_grabbed);
        }

        let now = std::time::Instant::now();
        let frame_dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        if self.cursor_grabbed {
            self.camera.update(input, frame_dt);
        }

        self.physics.step(frame_dt);

        let aspect = window.width() as f32 / window.height().max(1) as f32;
        *self
            .render_camera
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = self.camera.camera(aspect);

        self.physics.runtime.tick_only(&[render_stage]);
    }

    fn on_resized(&mut self, width: u32, height: u32) {
        self.physics.runtime.resize_all(width, height);
    }
}
