//! The viewer's window, GPU state and frame loop.

use meridian_sdk::assets::{
    AnimationPlayer, AnimationRepeat, JointPalette, ModelPiece, ModelRig, VertexSkin,
};
use meridian_sdk::{
    AppHandler, Background, Camera, Device, EnvironmentMap, FrameRecorder, GraphicsSession,
    InputState, Light, Motor3, Renderable3d, Scene3d, Sky, Vec3, Window, look_at_rotor,
    look_at_upright,
};

const SUN: Vec3 = Vec3 {
    x: -0.4,
    y: -1.0,
    z: -0.3,
};

pub(crate) struct App {
    tokio_runtime: tokio::runtime::Runtime,
    path: String,
    distance: f32,
    gpu: Option<GpuState>,
}

struct GpuState {
    base: GraphicsSession,
    scene: Scene3d,
    center: Vec3,
    radius: f32,
    pieces: Vec<ModelPiece>,
    rig: std::sync::Arc<ModelRig>,
    player: AnimationPlayer,
    /// Seconds an animation plays before the viewer crossfades to the next
    /// one (`MERIDIAN_CYCLE`), when the file has several.
    cycle: Option<f32>,
    since_switch: f32,
    last_frame: std::time::Instant,
    /// Scene time: the sum of every frame's step (see `fixed_dt`).
    sim_time: f32,
}

impl App {
    pub(crate) fn new(path: String, distance: f32) -> Self {
        Self {
            tokio_runtime: tokio::runtime::Runtime::new().expect("failed to start tokio runtime"),
            path,
            distance,
            gpu: None,
        }
    }
}

impl GpuState {
    fn build(mut base: GraphicsSession, path: &str) -> Self {
        let model = base.load_model_gltf(path).unwrap_or_else(|err| {
            meridian_sdk::log_error!("cannot load {path}: {err}");
            std::process::exit(1)
        });
        let (pieces, rig) = (model.pieces, model.rig);
        let animation = chosen_animation(&rig);
        let (center, radius) = bounds(&base, &pieces, &rig);
        meridian_sdk::log_info!("{path}: {} parts, radius {radius:.2}", pieces.len());
        let renderables = pieces
            .iter()
            .map(|piece| Renderable3d {
                mesh: piece.mesh,
                material: base.render.materials.register(piece.material),
                frame: Motor3::identity(),
                scale: Vec3::ONE,
                billboard: false,
                casts_shadow: true,
                palette: None,
                morph_weights: Vec::new(),
            })
            .collect();
        light_with_sky(&mut base);
        let scene = Scene3d {
            renderables,
            lights: vec![Light::Directional {
                direction: Motor3::from_rotation_translation(
                    look_at_rotor(Vec3::ZERO, SUN),
                    Vec3::ZERO,
                ),
                color: [1.0, 0.96, 0.9],
                intensity: 1.1,
            }],
            ambient_ground: [0.12, 0.1, 0.09],
            ambient_sky: [0.22, 0.25, 0.3],
            // The sky is the backdrop too; it falls back to the clear colour
            // when `MERIDIAN_ENV=none` leaves no environment.
            background: Background::Environment { blur: 0.0 },
            ..Scene3d::default()
        };
        Self {
            base,
            scene,
            center,
            radius,
            pieces,
            rig,
            player: AnimationPlayer::new(animation),
            cycle: cycle_seconds(),
            since_switch: 0.0,
            last_frame: std::time::Instant::now(),
            sim_time: 0.0,
        }
    }

    /// Moves the playing animation on by the time since the last frame and
    /// hands each skinned piece its joint matrices and each morphed one its weights.
    fn animate(&mut self) {
        let now = std::time::Instant::now();
        let delta = meridian_examples::app_main::fixed_dt()
            .unwrap_or_else(|| now.duration_since(self.last_frame).as_secs_f32());
        self.last_frame = now;
        self.sim_time += delta;
        self.cycle_animations(delta);
        self.player.advance(&self.rig, delta);
        let layers = self.player.layers();
        let palettes: Vec<JointPalette> = self
            .rig
            .matrices_mixed(&layers)
            .into_iter()
            .map(JointPalette::new)
            .collect();
        let morphs = self.rig.weights_mixed(&layers);
        let pieces = self.pieces.iter().zip(morphs);
        for (renderable, (piece, weights)) in self.scene.renderables.iter_mut().zip(pieces) {
            renderable.palette = piece
                .skeleton
                .and_then(|skeleton| palettes.get(skeleton).cloned());
            renderable.morph_weights = weights;
        }
    }
}

impl GpuState {
    /// Crossfades to the file's next animation every `cycle` seconds.
    fn cycle_animations(&mut self, delta: f32) {
        let count = self.rig.animations.len();
        self.since_switch += delta;
        let due = self
            .cycle
            .is_some_and(|period| count > 1 && self.since_switch >= period);
        if due {
            self.since_switch = 0.0;
            let next = self
                .player
                .animation()
                .map_or(0, |index| (index + 1) % count);
            self.player
                .crossfade_to(Some(next), 0.5, AnimationRepeat::Loop);
        }
    }
}

/// Seconds each animation plays before the next (`MERIDIAN_CYCLE`), if set.
fn cycle_seconds() -> Option<f32> {
    std::env::var("MERIDIAN_CYCLE")
        .ok()
        .and_then(|value| value.parse().ok())
}

/// The animation `MERIDIAN_ANIMATION` names (an index), else the first one
/// the file has.
fn chosen_animation(rig: &ModelRig) -> Option<usize> {
    let count = rig.animations.len();
    if count == 0 {
        return None;
    }
    let wanted = std::env::var("MERIDIAN_ANIMATION")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    meridian_sdk::log_info!("{count} animations, playing {}", wanted.min(count - 1));
    Some(wanted.min(count - 1))
}

/// The centre and radius of a sphere around every vertex of `pieces`, a
/// skinned piece posed at rest.
fn bounds(base: &GraphicsSession, pieces: &[ModelPiece], rig: &ModelRig) -> (Vec3, f32) {
    let rest = rig.matrices(None, 0.0);
    let (mut low, mut high) = ([f32::MAX; 3], [f32::MIN; 3]);
    for piece in pieces {
        let Some(source) = base.render.meshes.get(piece.mesh) else {
            continue;
        };
        let palette = piece
            .skeleton
            .and_then(|skeleton| rest.get(skeleton))
            .map(Vec::as_slice);
        for (vertex, position) in source.positions.iter().enumerate() {
            let position = match (palette, &source.skin) {
                (Some(palette), Some(skin)) => rest_position(palette, skin, vertex, *position),
                _ => *position,
            };
            for axis in 0..3 {
                low[axis] = low[axis].min(position[axis]);
                high[axis] = high[axis].max(position[axis]);
            }
        }
    }
    let center = Vec3::new(
        low[0].midpoint(high[0]),
        low[1].midpoint(high[1]),
        low[2].midpoint(high[2]),
    );
    let half = Vec3::new(high[0] - center.x, high[1] - center.y, high[2] - center.z);
    (
        center,
        (half.x * half.x + half.y * half.y + half.z * half.z)
            .sqrt()
            .max(0.01),
    )
}

/// `position` of `vertex` moved by the weighted joint matrices of `palette`.
fn rest_position(
    palette: &[[[f32; 4]; 4]],
    skin: &VertexSkin,
    vertex: usize,
    position: [f32; 3],
) -> [f32; 3] {
    let mut moved = [0.0; 3];
    for (joint, weight) in skin.joints[vertex].iter().zip(skin.weights[vertex]) {
        let Some(matrix) = palette.get(usize::from(*joint)) else {
            continue;
        };
        for (row, out) in moved.iter_mut().enumerate() {
            let sum = matrix[0][row] * position[0]
                + matrix[1][row] * position[1]
                + matrix[2][row] * position[2]
                + matrix[3][row];
            *out += weight * sum;
        }
    }
    moved
}

impl AppHandler for App {
    fn on_ready(&mut self, window: &Window) {
        let target = window.surface_target();
        let (width, height) = (window.width(), window.height());
        let (device, surface) = self
            .tokio_runtime
            .block_on(Device::new_windowed(target, width, height))
            .expect("failed to create windowed GPU device");
        let base = GraphicsSession::new(device, surface, width, height);
        self.gpu = Some(GpuState::build(base, &self.path));
    }

    fn on_redraw(&mut self, window: &Window, _input: &InputState) {
        let Some(gpu) = &mut self.gpu else {
            return;
        };
        gpu.animate();
        let angle = gpu.sim_time * 0.4;
        let distance = gpu.radius * self.distance;
        let eye = gpu.center
            + Vec3::new(
                distance * angle.sin(),
                gpu.radius * 0.5 * self.distance.min(1.0),
                distance * angle.cos(),
            );
        #[expect(
            clippy::cast_precision_loss,
            reason = "window sizes are far below 2^24"
        )]
        let aspect = window.width() as f32 / window.height().max(1) as f32;
        gpu.scene.camera = Camera {
            frame: Motor3::from_rotation_translation(look_at_upright(eye, gpu.center), eye),
            projection: meridian_sdk::Projection::perspective(
                1.0,
                aspect,
                gpu.radius * 0.02,
                gpu.radius * 20.0,
            ),
        };
        meridian_sdk::render_frame(&mut gpu.base, &gpu.scene, [0.45, 0.55, 0.7, 1.0], window);
    }

    fn on_resized(&mut self, width: u32, height: u32) {
        if let Some(gpu) = &mut self.gpu {
            gpu.base.resize(width, height);
        }
    }
}

/// Whether `path` names a Radiance panorama.
fn is_hdr_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("hdr"))
}

/// Loads the environment a `MERIDIAN_ENV` value names: six comma-separated
/// cube faces (`+x,-x,+y,-y,+z,-z`, ADR 038) or one `<file>.hdr` panorama
/// (ADR 036). `None` when the value names neither.
fn load_named_environment(base: &mut GraphicsSession, value: &str) -> Option<Result<(), String>> {
    if let Ok(faces) = <[&str; 6]>::try_from(value.split(',').collect::<Vec<_>>()) {
        return Some(base.load_environment_cube(faces));
    }
    is_hdr_path(value).then(|| base.load_environment(value))
}

/// Lights the model with a procedural sky (ADR 035) whose sun sits where the
/// directional light does; `MERIDIAN_ENV=none` keeps the ambient colours alone,
/// and any other `MERIDIAN_ENV` that [`load_named_environment`] understands
/// lights it with that image instead.
fn light_with_sky(base: &mut GraphicsSession) {
    if let Ok(value) = std::env::var("MERIDIAN_ENV") {
        if value == "none" {
            return;
        }
        match load_named_environment(base, &value) {
            Some(Ok(())) => return,
            Some(Err(err)) => meridian_sdk::log_error!("environment: {err}"),
            None => {}
        }
    }
    let sky = Sky {
        sun_direction: [-SUN.x, -SUN.y, -SUN.z],
        sun_radiance: [30.0, 27.0, 22.0],
        sun_radius: 0.06,
        ..Sky::default()
    };
    match EnvironmentMap::from_sky(&sky) {
        Ok(map) => base.render.recorder.set_environment(Some(&map)),
        Err(err) => meridian_sdk::log_error!("sky environment: {err}"),
    }
}
