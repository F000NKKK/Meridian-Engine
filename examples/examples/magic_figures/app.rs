//! The application handler: window, GPU state and the frame loop.

use meridian_examples::paths::asset_path;
use meridian_sdk::{
    AppHandler, Device, FlyCamera, GraphicsSession, InputState, KeyCode, Light, Listener, Material,
    Motor3, Renderable3d, Rotor, Scene3d, Vec3, Window, fly_controls, ground_mesh_source,
    look_at_rotor,
};

use crate::{
    Hud, MusicRig, ORBIT_RADIUS, SUN_DIRECTION, ShapeSpec, load_scene, mesh_source_for,
    orbit_position, scene_extras,
};

pub(crate) struct App {
    tokio_runtime: tokio::runtime::Runtime,
    camera: FlyCamera,
    shapes: Vec<ShapeSpec>,
    music: Option<MusicRig>,
    cursor_grabbed: bool,
    last_frame: std::time::Instant,
    sim_time: f32,
    gpu: Option<GpuState>,
}

struct GpuState {
    base: GraphicsSession,
    scene: Scene3d,
    hud: Hud,
    /// Per-shape index into `scene.renderables`, in the same order as
    /// `App::shapes`, so `on_redraw` can update each shape's frame every
    /// tick without re-walking `shapes`.
    shape_renderable_indices: Vec<usize>,
    /// Index into `scene.lights` of each shape's own `Light::Point`, so
    /// `on_redraw` can move it along with the shape every frame.
    point_light_indices: Vec<usize>,
}

impl App {
    pub(crate) fn new() -> Self {
        let tokio_runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        let shapes = load_scene();
        let music = match tokio_runtime.block_on(MusicRig::load(&shapes)) {
            Ok(music) => Some(music),
            Err(err) => {
                meridian_sdk::log_warn!("running silent: {err}");
                None
            }
        };
        Self {
            tokio_runtime,
            camera: FlyCamera::new(Vec3::new(0.0, 2.0, 7.0)),
            shapes,
            music,
            cursor_grabbed: true,
            last_frame: std::time::Instant::now(),
            sim_time: 0.0,
            gpu: None,
        }
    }
}

impl GpuState {
    /// Registers the floor, every shape and the sun, and lights them.
    fn build(mut base: GraphicsSession, shapes: &[ShapeSpec]) -> Self {
        let mut renderables = vec![floor_renderable(&mut base)];
        let mut shape_renderable_indices = Vec::with_capacity(shapes.len());
        for shape in shapes {
            shape_renderable_indices.push(renderables.len());
            renderables.push(shape_renderable(&mut base, shape));
        }
        renderables.extend(scene_extras(&mut base));

        let (lights, point_light_indices) = scene_lights(shapes);
        let scene = Scene3d {
            renderables,
            lights,
            ambient_ground: [0.012, 0.012, 0.016],
            ambient_sky: [0.02, 0.02, 0.028],
            ..Scene3d::default()
        };
        let hud = Hud::new(&mut base);
        Self {
            base,
            scene,
            hud,
            shape_renderable_indices,
            point_light_indices,
        }
    }
}

/// The textured floor.
fn floor_renderable(base: &mut GraphicsSession) -> Renderable3d {
    let floor_texture = base.load_texture(&asset_path("assets/textures/floor.png"));
    let material = base.render.materials.register(Material {
        albedo: Some(floor_texture),
        base_color_factor: [1.0, 1.0, 1.0, 1.0],
        ..Default::default()
    });
    let mesh = base
        .render
        .meshes
        .register(ground_mesh_source(14.0, 10.0))
        .expect("floor mesh must be valid");
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

/// One orbiting shape at its starting position.
fn shape_renderable(base: &mut GraphicsSession, shape: &ShapeSpec) -> Renderable3d {
    let mesh = base
        .render
        .meshes
        .register(mesh_source_for(shape))
        .unwrap_or_else(|e| panic!("{} mesh must be valid: {e}", shape.name));
    let texture = base.load_texture(&asset_path(&shape.texture_file));
    let material = base.render.materials.register(Material {
        albedo: Some(texture),
        base_color_factor: [
            shape.glow_color[0],
            shape.glow_color[1],
            shape.glow_color[2],
            1.0,
        ],
        // Unlit: each shape reads as glowing regardless of the
        // directional light below, and its emissive value is
        // what BloomPass blooms — see the module doc.
        unlit: true,
        emissive: shape.glow_color,
        ..Default::default()
    });
    Renderable3d {
        mesh,
        material,
        frame: Motor3::translation(orbit_position(shape, 0.0)),
        billboard: false,
        casts_shadow: true,
        palette: None,
        morph_weights: Vec::new(),
    }
}

/// The sun plus one `Light::Point` per shape, in its own glow color,
/// colocated with the shape — this is what actually casts colored light
/// onto the floor and the other shapes as they orbit; the material's
/// `emissive` alone (feeding `BloomPass`) only makes the shape itself
/// glow, it doesn't illuminate anything nearby. `MAX_LIGHTS = 4` is
/// exactly 1 directional + 3 shapes, no headroom to spare. Also returns
/// each shape's index into the light list.
fn scene_lights(shapes: &[ShapeSpec]) -> (Vec<Light>, Vec<usize>) {
    let mut lights = vec![Light::Directional {
        direction: Motor3::from_rotation_translation(
            look_at_rotor(Vec3::ZERO, SUN_DIRECTION),
            Vec3::ZERO,
        ),
        color: [1.0, 0.96, 0.9],
        intensity: 0.7,
    }];
    let mut point_light_indices = Vec::with_capacity(shapes.len());
    for shape in shapes {
        point_light_indices.push(lights.len());
        lights.push(Light::Point {
            position: Motor3::translation(orbit_position(shape, 0.0)),
            color: shape.glow_color,
            intensity: 2.5,
            range: ORBIT_RADIUS * 2.2,
        });
    }
    (lights, point_light_indices)
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
        let base = GraphicsSession::new(device, surface, width, height);
        self.gpu = Some(GpuState::build(base, &self.shapes));
    }

    fn on_redraw(&mut self, window: &Window, input: &InputState) {
        let Some(gpu) = &mut self.gpu else {
            return;
        };

        if input.was_key_pressed(KeyCode::Escape) {
            self.cursor_grabbed = !self.cursor_grabbed;
            window.set_cursor_grabbed(self.cursor_grabbed);
        }

        let now = std::time::Instant::now();
        let frame_dt = meridian_examples::app_main::fixed_dt()
            .unwrap_or_else(|| (now - self.last_frame).as_secs_f32().min(0.1));
        self.last_frame = now;
        if self.cursor_grabbed {
            self.camera.update(fly_controls(input), frame_dt);
        }

        self.sim_time += frame_dt;
        let elapsed = self.sim_time;
        let mut positions = Vec::with_capacity(self.shapes.len());
        for (i, shape) in self.shapes.iter().enumerate() {
            let position = orbit_position(shape, elapsed);
            positions.push(position);
            let spin = Rotor::from_axis_angle(Vec3::Y, elapsed * shape.spin_speed);
            let renderable = &mut gpu.scene.renderables[gpu.shape_renderable_indices[i]];
            renderable.frame = Motor3::from_rotation_translation(spin, position);
            if let Light::Point {
                position: light_position,
                ..
            } = &mut gpu.scene.lights[gpu.point_light_indices[i]]
            {
                *light_position = Motor3::translation(position);
            }
        }

        let aspect = window.width() as f32 / window.height().max(1) as f32;
        let camera = self.camera.camera(aspect);
        if let Some(music) = &mut self.music {
            music.refill(
                &Listener {
                    frame: camera.frame,
                },
                &positions,
            );
        }
        gpu.scene.camera = camera;

        let hud = gpu.hud.overlay([window.width(), window.height()], elapsed);
        meridian_sdk::compose_frame(
            &mut gpu.base,
            &gpu.scene,
            &[hud],
            [0.03, 0.03, 0.05, 1.0],
            window,
        );
    }

    fn on_resized(&mut self, width: u32, height: u32) {
        if let Some(gpu) = &mut self.gpu {
            gpu.base.resize(width, height);
        }
    }
}
