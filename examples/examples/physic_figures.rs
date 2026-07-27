//! Real rigid-body physics: a sphere, a cube and a pyramid dropped above
//! a textured floor and stepped every frame through a real
//! `meridian_sdk::Runtime` (`engine-core`'s single `JobGraph`-based
//! frame-work entry point — see that crate's own module doc). Physics
//! *and* rendering both go through this one `Runtime`: a registered
//! [`PhysicsStepStage`] (driven by the fixed-timestep accumulator, see
//! [`PhysicsRig::step`]) and a registered `meridian_sdk::RenderStage`
//! (driven once per redraw, depending on the physics stage) — no raw,
//! runtime-bypassing pipeline call for either. See
//! `Runtime::tick_only`'s own doc comment for why physics and render
//! need *selective* ticks (different multiplicities per real display
//! frame) rather than one `Runtime::tick()` running both together every
//! time.
//!
//! **Scene composition itself lives in
//! `assets/scenes/physic_figures.mel`**, parsed through
//! `meridian_sdk::dsl` (via this workspace's shared
//! `meridian_examples::scene_loader`) — every entity's position, mesh
//! shape, texture and collider is data, not Rust code; this file only
//! reads it once ([`load_scene`]) and turns it into real physics bodies
//! ([`PhysicsRig::new`]) and renderables (the `RenderStage`'s
//! `build_scene` closure, registered in
//! [`AppHandler::on_ready`]). What stays genuine Rust logic (per this
//! workspace's rule that the DSL describes composition, not behavior):
//! the fixed-timestep accumulator, the free-fly camera, and each
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
//! emissive glow here beyond the visible "sun" sphere — these are
//! ordinary lit, textured physics props, not the "magic" glowing
//! shapes.
//!
//! This example depends on `meridian-sdk` alone (plus `tokio`, for its
//! own async GPU-device handshake) — every type below is reached
//! through `meridian_sdk`'s re-exports, never through
//! `meridian-gac-core`/`meridian-physics-core`/`meridian-graphics-core`/
//! etc. directly.
//!
//! Run with:
//!   ./build.sh run physic_figures

use std::sync::{Arc, Mutex};

use meridian_examples::paths::asset_path;
use meridian_examples::scene_loader::load_dsl_scene;
use meridian_sdk::dsl;
use meridian_sdk::{
    AppHandler, AudioSubsystem, Camera, ColliderShape, ConstraintSolver, Device, FlyCamera,
    GraphicsBase, InputState, KeyCode, Light, Material, MaterialHandle, MeshHandle, Mixer, Motor3,
    PhysicsComputeStepStage, PhysicsSubsystem, RenderStage, Renderable3D, RigidBody, Runtime,
    Scene3D, SpeakerLayout, StageContext, StageId, Vec3, Window, cube_mesh_source,
    ground_mesh_source, icosphere_mesh_source, look_at_rotor, pyramid_mesh_source,
    run_windowed_app,
};

/// The one directional light's travel direction — shared by the light
/// itself and the visible "sun" sphere ([`sun_renderable`]) placed
/// opposite it in the sky, so it's never ambiguous where the shadows'
/// light is coming from.
const SUN_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.3);
/// How far above/behind the scene the visible sun sphere sits — far
/// enough that its own shadow (it's a real renderable, drawn like
/// anything else) never reaches the play area, close enough (with
/// [`SUN_VISUAL_RADIUS`]) to read as an unmistakable bright disc rather
/// than a barely-visible dot once the camera pans toward it.
const SUN_DISTANCE: f32 = 35.0;
const SUN_VISUAL_RADIUS: f32 = 6.0;

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
/// — identical for most shapes, except two disclosed per-shape offsets:
/// the floor mesh is a thin visual quad at `y = 0`, rendered at
/// identity regardless of its (thicker, lower) collider's frame; the
/// pyramid mesh's origin is its own base center, but its `Cuboid`
/// collider is centered on the body, so the mesh renders shifted down
/// by the collider's half-height to keep the base flush with the
/// resting contact point. Takes `mesh_shape`/`collider` directly
/// (rather than a whole `&SceneEntity`) so it works equally for the
/// load-time [`SceneEntity`] list and the [`RenderEntity`] list the
/// render stage's closure actually reads from.
fn mesh_render_frame(mesh_shape: &str, collider: ColliderShape, body_frame: Motor3) -> Motor3 {
    match mesh_shape {
        "ground" => Motor3::identity(),
        "pyramid" => {
            let ColliderShape::Cuboid { half_extents } = collider else {
                panic!("pyramid entity must have a cuboid collider");
            };
            body_frame.compose(Motor3::translation(Vec3::new(0.0, -half_extents.y, 0.0)))
        }
        _ => body_frame,
    }
}

/// A small, always-unlit-and-glowing sphere placed opposite
/// [`SUN_DIRECTION`] at [`SUN_DISTANCE`] — makes the scene's one
/// directional light have an obvious, visible source instead of coming
/// from nowhere. Purely decorative (its own mesh/material, no physics
/// body, not part of [`SceneEntity`]) — it happens to fall inside the
/// shadow camera's fixed volume like everything else, but at that
/// distance and size it never occludes anything the play area's shadow
/// would need.
fn sun_renderable(base: &mut GraphicsBase) -> Renderable3D {
    let mesh = base
        .meshes
        .register(icosphere_mesh_source(1, SUN_VISUAL_RADIUS))
        .expect("sun mesh must be valid");
    let material = base.materials.register(Material {
        base_color_factor: [1.0, 0.95, 0.8, 1.0],
        unlit: true,
        emissive: [1.0, 0.95, 0.8],
        ..Default::default()
    });
    // The real light's direction is mostly straight down (`SUN_DIRECTION.y
    // = -1.0` dominates), which would put a physically-exact sun almost
    // directly overhead — technically correct, but the free-fly camera
    // starts at pitch `0` (looking at the horizon), so a new viewer
    // would have to already know to pitch steeply upward to ever find
    // it. This flattens the *visual* placement's elevation (still in
    // the light's general horizontal direction, just not as steep) so
    // it's findable by panning around near the horizon instead — a
    // deliberate, disclosed fudge between "physically exact" and
    // "a first-time viewer can actually find it."
    let visual_direction = Vec3::new(SUN_DIRECTION.x, -0.5, SUN_DIRECTION.z).normalize();
    let position = visual_direction * -SUN_DISTANCE;
    Renderable3D {
        mesh,
        material,
        frame: Motor3::translation(position),
        billboard: false,
        // Sitting along the light's own incoming direction, this would
        // otherwise occlude the light for the whole scene below it —
        // see `Renderable3D::casts_shadow`'s own doc comment.
        casts_shadow: false,
    }
}

/// One [`SceneEntity`]'s registered GPU handles plus the bits
/// [`mesh_render_frame`] needs — built once in [`AppHandler::on_ready`],
/// moved into the render stage's `build_scene` closure so it never
/// re-registers a mesh/material or re-reads `scene_entities` per frame.
struct RenderEntity {
    mesh_shape: String,
    collider: ColliderShape,
    mesh: MeshHandle,
    material: MaterialHandle,
}

/// Fixed-timestep driver around a real `meridian_sdk::Runtime` running
/// one registered [`PhysicsStepStage`] — the physics stepping itself
/// (integrate/relax-contacts/resolve) is no longer hand-rolled here; see
/// `PhysicsSubsystem::step`'s own doc comment for the multi-point-
/// manifold relaxation it does internally. What's left at this layer is
/// deliberately application policy, not engine plumbing: the render
/// loop's `frame_dt` varies with frame rate, but the solver is only
/// validated at a constant [`PHYSICS_DT`], so this accumulates
/// wall-clock time and calls `Runtime::tick_only` (just the physics
/// stage — see that method's own doc for why not plain `tick`) once per
/// whole [`PHYSICS_DT`] increment, capped so a stall (e.g. window drag)
/// can't spiral into running hundreds of catch-up ticks at once. Also
/// owns the `Runtime` the render stage gets registered onto in
/// [`AppHandler::on_ready`] — physics and render share one `Runtime`,
/// per this module's own top-level doc.
struct PhysicsRig {
    runtime: Runtime,
    physics_stage: StageId,
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

        let physics = PhysicsSubsystem {
            bodies,
            solver: ConstraintSolver::new(SOLVER_RESTITUTION).with_friction(SOLVER_FRICTION),
            ..Default::default()
        };
        // No audio in this example — `Runtime::new` still requires an
        // `AudioSubsystem` (physics and audio are the two subsystems
        // every `Runtime` owns, per `engine-core`'s own module doc),
        // left unused here.
        let audio = AudioSubsystem::new(Mixer::new(SpeakerLayout::mono()));

        let mut runtime = Runtime::new(physics, audio);
        let physics_stage = runtime.add_stage("physics", &[], PhysicsStepStage::new(PHYSICS_DT));

        Self {
            runtime,
            physics_stage,
            accumulator: 0.0,
        }
    }

    fn step(&mut self, frame_dt: f32) {
        self.accumulator += frame_dt;
        let mut steps = 0;
        while self.accumulator >= PHYSICS_DT && steps < 8 {
            self.runtime.tick_only(&[self.physics_stage]);
            self.accumulator -= PHYSICS_DT;
            steps += 1;
        }
    }
}

struct App {
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
            render_camera: Arc::new(Mutex::new(Camera::default())),
            render_stage: None,
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

        let render_entities: Vec<RenderEntity> = self
            .scene_entities
            .iter()
            .map(|entity| {
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
            })
            .collect();
        let sun = sun_renderable(&mut base);

        let lights = vec![Light::Directional {
            direction: Motor3::from_rotation_translation(
                look_at_rotor(Vec3::ZERO, SUN_DIRECTION),
                Vec3::ZERO,
            ),
            color: [1.0, 0.96, 0.9],
            intensity: 1.1,
        }];

        let render_camera = self.render_camera.clone();
        let render_stage = RenderStage::new(
            base,
            window.clone(),
            [0.05, 0.05, 0.08, 1.0],
            move |ctx: &StageContext| {
                let bodies = &ctx.physics().bodies;
                let mut renderables: Vec<Renderable3D> = render_entities
                    .iter()
                    .zip(bodies.iter())
                    .map(|(re, body)| Renderable3D {
                        mesh: re.mesh,
                        material: re.material,
                        frame: mesh_render_frame(&re.mesh_shape, re.collider, body.frame),
                        billboard: false,
                        casts_shadow: true,
                    })
                    .collect();
                renderables.push(sun.clone());
                Scene3D {
                    renderables,
                    lights: lights.clone(),
                    camera: *render_camera.lock().unwrap(),
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
        *self.render_camera.lock().unwrap() = self.camera.camera(aspect);

        self.physics.runtime.tick_only(&[render_stage]);
    }

    fn on_resized(&mut self, width: u32, height: u32) {
        self.physics.runtime.resize_all(width, height);
    }
}

fn main() {
    meridian_examples::app_main::install_diagnostics("physic_figures");
    run_windowed_app("Meridian Engine — Physic Figures", 1024, 768, App::new())
        .expect("windowed app exited with an error");
}
