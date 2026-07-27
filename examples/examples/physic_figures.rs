//! Real rigid-body physics: a sphere, a cube and a pyramid dropped above
//! a textured floor and stepped every frame through a real
//! `meridian_sdk::Runtime` (`engine-core`'s single `JobGraph`-based
//! frame-work entry point — see that crate's own module doc) via a
//! registered [`PhysicsStepStage`] and [`Runtime::tick`] — the first
//! proof in this workspace that `Runtime` composes end-to-end in a real
//! windowed application, not just in `engine-core`'s own unit tests.
//!
//! **Scene composition itself lives in
//! `assets/scenes/physic_figures.mel`**, parsed through
//! `meridian_sdk::dsl` (via this workspace's shared
//! `meridian_examples::scene_loader`) — every entity's position, mesh
//! shape, texture and collider is data, not Rust code; this file only
//! reads it once ([`load_scene`]) and turns it into real physics bodies
//! ([`PhysicsRig::new`]) and renderables ([`AppHandler::on_ready`]).
//! What stays genuine Rust logic (per this workspace's rule that the
//! DSL describes composition, not behavior): the fixed-timestep
//! accumulator ([`PhysicsRig::step`]), the free-fly camera, and each
//! shape's own render-frame quirk (the floor mesh is a thin visual quad
//! rendered at identity while its *collider* is a thicker slab; the
//! pyramid mesh's origin is its base center while its *collider* is
//! centered on the body — both disclosed per-shape offsets in
//! [`mesh_render_frame`], the same simplification the hand-written
//! version had, just no longer duplicated as four hardcoded literals).
//!
//! `physics-core` only has two collider shapes today, `Sphere` and
//! `Cuboid` — there is no dedicated pyramid collider, so the DSL scene
//! gives the pyramid entity a `Cuboid` collider roughly bounding its
//! mesh (it settles and rests like a box).
//!
//! Shares its base with `magic_figures` (`meridian_sdk::scene`'s
//! `GraphicsBase`): same mesh builders, same lighting model. No bloom
//! emissive glow here — these are ordinary lit, textured physics props,
//! not the "magic" glowing shapes.
//!
//! This example depends on `meridian-sdk` alone (plus `tokio`, for its
//! own async GPU-device handshake) — every type below is reached
//! through `meridian_sdk`'s re-exports, never through
//! `meridian-gac-core`/`meridian-physics-core`/`meridian-graphics-core`/
//! etc. directly.
//!
//! Run with:
//!   ./build.sh run physic_figures

use meridian_examples::paths::asset_path;
use meridian_examples::scene_loader::load_dsl_scene;
use meridian_sdk::dsl;
use meridian_sdk::{
    AppHandler, AudioSubsystem, ColliderShape, ConstraintSolver, Device, FlyCamera, GraphicsBase,
    InputState, KeyCode, Light, Material, Mixer, Motor3, PhysicsStepStage, PhysicsSubsystem,
    Renderable3D, RigidBody, Runtime, Scene3D, SpeakerLayout, Vec3, Window, cube_mesh_source,
    ground_mesh_source, icosphere_mesh_source, look_at_rotor, pyramid_mesh_source,
    run_windowed_app,
};

const PHYSICS_DT: f32 = 1.0 / 60.0;
/// `0`: a settled body must not bounce at all. Combined with
/// `ConstraintSolver`'s `restitution_velocity_threshold` (which already
/// suppresses bounce below a small closing speed regardless of this
/// value), this makes landing fully inelastic — no perpetual "settles,
/// then bounces off residual gravity velocity" jitter, ever.
const SOLVER_RESTITUTION: f32 = 0.0;
/// Coulomb friction coefficient — without this, resting bodies had
/// nothing slowing lateral sliding, so any small rotational/positional
/// jitter (see `NarrowPhase`'s box-box manifold) could slide the box
/// across the floor indefinitely instead of settling.
const SOLVER_FRICTION: f32 = 0.6;

/// One `<Entity>` from `assets/scenes/physic_figures.mel`, flattened
/// out of its typed `<Transform>`/`<Mesh>`/`<Material>`/`<RigidBody>`
/// children into the plain fields this example actually needs — the
/// DSL tree itself is only walked once, here, in [`load_scene`].
struct SceneEntity {
    position: Vec3,
    mesh_shape: String,
    size: f32,
    size2: f32,
    texture: String,
    mass: f32,
    collider: ColliderShape,
}

/// Parses `assets/scenes/physic_figures.mel` against
/// `meridian_sdk::dsl::default_registry()` (this scene only uses
/// built-in tags — no custom `#[dsl_tag]` needed here, unlike
/// `magic_figures`) and flattens every `<Entity>` into a [`SceneEntity`].
/// The read/parse/error-logging sequence itself is
/// `meridian_examples::scene_loader::load_dsl_scene` — shared with
/// every other example, since it's identical regardless of which tags
/// a scene uses.
fn load_scene() -> Vec<SceneEntity> {
    let registry = dsl::default_registry();
    let root = load_dsl_scene("assets/scenes/physic_figures.mel", &registry);

    root.children
        .iter()
        .map(|entity_node| {
            let entity = entity_node
                .downcast_ref::<dsl::Entity>()
                .unwrap_or_else(|| panic!("<{}> at scene root must be <Entity>", entity_node.tag));

            let mut position = Vec3::ZERO;
            let mut mesh_shape = String::new();
            let mut size = 0.0f32;
            let mut size2 = 0.0f32;
            let mut texture = String::new();
            let mut mass = 0.0f32;
            let mut collider = ColliderShape::Sphere { radius: 0.0 };

            for child in &entity_node.children {
                if let Some(t) = child.downcast_ref::<dsl::Transform>() {
                    position = Vec3::new(t.x, t.y, t.z);
                } else if let Some(m) = child.downcast_ref::<dsl::Mesh>() {
                    mesh_shape = m
                        .shape
                        .clone()
                        .unwrap_or_else(|| panic!("{}: <Mesh> needs a 'shape'", entity.name));
                    size = m
                        .size
                        .unwrap_or_else(|| panic!("{}: <Mesh> needs a 'size'", entity.name));
                    size2 = m.size2.unwrap_or(0.0);
                } else if let Some(mat) = child.downcast_ref::<dsl::Material>() {
                    texture = mat
                        .texture
                        .clone()
                        .unwrap_or_else(|| panic!("{}: <Material> needs a 'texture'", entity.name));
                } else if let Some(rb) = child.downcast_ref::<dsl::RigidBody>() {
                    mass = rb.mass;
                    collider = match rb.shape.as_str() {
                        "sphere" => ColliderShape::Sphere {
                            radius: rb.radius.unwrap_or_else(|| {
                                panic!("{}: sphere <RigidBody> needs 'radius'", entity.name)
                            }),
                        },
                        "cuboid" => ColliderShape::Cuboid {
                            half_extents: Vec3::new(
                                rb.hx.unwrap_or_else(|| {
                                    panic!("{}: cuboid <RigidBody> needs 'hx'", entity.name)
                                }),
                                rb.hy.unwrap_or_else(|| {
                                    panic!("{}: cuboid <RigidBody> needs 'hy'", entity.name)
                                }),
                                rb.hz.unwrap_or_else(|| {
                                    panic!("{}: cuboid <RigidBody> needs 'hz'", entity.name)
                                }),
                            ),
                        },
                        other => panic!("{}: unknown RigidBody shape '{other}'", entity.name),
                    };
                }
            }

            SceneEntity {
                position,
                mesh_shape,
                size,
                size2,
                texture,
                mass,
                collider,
            }
        })
        .collect()
}

/// Builds the real `MeshSource` for one [`SceneEntity`], dispatching on
/// its `mesh_shape`/`size`/`size2` — see `dsl::Mesh`'s own doc comment
/// for what each shape's `size`/`size2` mean.
fn mesh_source_for(entity: &SceneEntity) -> meridian_sdk::MeshSource {
    match entity.mesh_shape.as_str() {
        "ground" => ground_mesh_source(entity.size, entity.size2),
        "sphere" => icosphere_mesh_source(2, entity.size),
        "cube" => cube_mesh_source(entity.size),
        "pyramid" => pyramid_mesh_source(entity.size, entity.size2),
        other => panic!("unknown mesh shape '{other}'"),
    }
}

/// This shape's rendered frame, given its current physics `body_frame`
/// — identical for most shapes, except two disclosed per-shape offsets
/// (both already present in the DSL-free version of this example, just
/// no longer duplicated as four hardcoded call sites): the floor mesh
/// is a thin visual quad at `y = 0`, rendered at identity regardless of
/// its (thicker, lower) collider's frame; the pyramid mesh's origin is
/// its own base center, but its `Cuboid` collider is centered on the
/// body, so the mesh renders shifted down by the collider's
/// half-height to keep the base flush with the resting contact point.
fn mesh_render_frame(entity: &SceneEntity, body_frame: Motor3) -> Motor3 {
    match entity.mesh_shape.as_str() {
        "ground" => Motor3::identity(),
        "pyramid" => {
            let ColliderShape::Cuboid { half_extents } = entity.collider else {
                panic!("pyramid entity must have a cuboid collider");
            };
            body_frame.compose(Motor3::translation(Vec3::new(0.0, -half_extents.y, 0.0)))
        }
        _ => body_frame,
    }
}

/// Fixed-timestep driver around a real `meridian_sdk::Runtime` — the
/// physics stepping itself (integrate/relax-contacts/resolve) is no
/// longer hand-rolled here; see `PhysicsSubsystem::step`'s own doc
/// comment for the multi-point-manifold relaxation it does internally.
/// What's left at this layer is deliberately application policy, not
/// engine plumbing: the render loop's `frame_dt` varies with frame
/// rate, but the solver is only validated at a constant [`PHYSICS_DT`]
/// (see `Runtime::tick_fixed`'s own doc for why that method exists
/// instead of `Runtime::tick`'s wall-clock-driven step), so this
/// accumulates wall-clock time and calls [`Runtime::tick_fixed`] once
/// per whole [`PHYSICS_DT`] increment, capped so a stall (e.g. window
/// drag) can't spiral into running hundreds of catch-up ticks at once.
struct PhysicsRig {
    runtime: Runtime,
    accumulator: f32,
}

impl PhysicsRig {
    fn new(scene: &[SceneEntity]) -> Self {
        let bodies = scene
            .iter()
            .map(|entity| RigidBody {
                frame: Motor3::translation(entity.position),
                mass: entity.mass,
                shape: entity.collider,
                ..Default::default()
            })
            .collect();

        // No audio in this example — `SubsystemManager::new` still
        // requires a `Mixer` (physics and audio are the two subsystems
        // every `Runtime` owns, per `engine-core`'s own module doc),
        // left unused here.
        let subsystems = SubsystemManager {
            physics: PhysicsSubsystem {
                bodies,
                solver: ConstraintSolver::new(SOLVER_RESTITUTION).with_friction(SOLVER_FRICTION),
                ..Default::default()
            },
            ..SubsystemManager::new(Mixer::new(SpeakerLayout::mono()))
        };

        Self {
            runtime: Runtime::new(subsystems),
            accumulator: 0.0,
        }
    }

    fn bodies(&self) -> Vec<RigidBody> {
        self.runtime.subsystems.physics.bodies.clone()
    }

    fn step(&mut self, frame_dt: f32) {
        self.accumulator += frame_dt;
        let mut steps = 0;
        while self.accumulator >= PHYSICS_DT && steps < 8 {
            self.runtime.tick_fixed(PHYSICS_DT);
            self.accumulator -= PHYSICS_DT;
            steps += 1;
        }
    }
}

struct GpuState {
    base: GraphicsBase,
    scene: Scene3D,
}

struct App {
    camera: FlyCamera,
    cursor_grabbed: bool,
    last_frame: std::time::Instant,
    scene_entities: Vec<SceneEntity>,
    physics: PhysicsRig,
    tokio_runtime: tokio::runtime::Runtime,
    gpu: Option<GpuState>,
}

impl App {
    fn new() -> Self {
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
            gpu: None,
        }
    }
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
        let mut base = GraphicsBase::new(device, surface, width, height);

        let bodies = self.physics.bodies();
        let renderables: Vec<Renderable3D> = self
            .scene_entities
            .iter()
            .zip(&bodies)
            .map(|(entity, body)| {
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
                Renderable3D {
                    mesh,
                    material,
                    frame: mesh_render_frame(entity, body.frame),
                    billboard: false,
                }
            })
            .collect();

        let scene = Scene3D {
            renderables,
            lights: vec![Light::Directional {
                direction: Motor3::from_rotation_translation(
                    look_at_rotor(Vec3::ZERO, Vec3::new(-0.4, -1.0, -0.3)),
                    Vec3::ZERO,
                ),
                color: [1.0, 0.96, 0.9],
                intensity: 1.1,
            }],
            ambient_ground: [0.08, 0.08, 0.09],
            ambient_sky: [0.12, 0.12, 0.15],
            ..Scene3D::default()
        };

        self.gpu = Some(GpuState { base, scene });
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
        let frame_dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        if self.cursor_grabbed {
            self.camera.update(input, frame_dt);
        }

        self.physics.step(frame_dt);
        let bodies = self.physics.bodies();
        for ((entity, body), renderable) in self
            .scene_entities
            .iter()
            .zip(&bodies)
            .zip(&mut gpu.scene.renderables)
        {
            renderable.frame = mesh_render_frame(entity, body.frame);
        }

        let aspect = window.width() as f32 / window.height().max(1) as f32;
        gpu.scene.camera = self.camera.camera(aspect);

        meridian_examples::render::render_frame(
            &mut gpu.base,
            &gpu.scene,
            [0.05, 0.05, 0.08, 1.0],
            window,
        );
    }

    fn on_resized(&mut self, width: u32, height: u32) {
        if let Some(gpu) = &mut self.gpu {
            gpu.base.resize(width, height);
        }
    }
}

fn main() {
    meridian_examples::app_main::install_diagnostics("physic_figures");
    run_windowed_app("Meridian Engine — Physic Figures", 1024, 768, App::new())
        .expect("windowed app exited with an error");
}
