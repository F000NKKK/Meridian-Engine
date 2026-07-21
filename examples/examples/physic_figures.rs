//! Real rigid-body physics: a sphere, a cube and a pyramid dropped above
//! a textured floor and stepped through `physics-core`'s
//! `Integrator`/`BroadPhase`/`NarrowPhase`/`ConstraintSolver` every
//! frame, each body's resulting `Motor3` frame fed straight to its
//! `Renderable3D` — no separate "visual" transform, physics *is* the
//! transform.
//!
//! `physics-core` only has two collider shapes today, `Sphere` and
//! `Cuboid` (see `ColliderShape`) — there is no dedicated pyramid
//! collider. The pyramid body uses a `Cuboid` collider sized to roughly
//! bound it (a disclosed simplification: it settles and rests like a
//! box, while the mesh drawn at its frame is the real pyramid shape) —
//! see [`pyramid_collider_half_extents`].
//!
//! Shares its base with `magic_figures` (`examples::scene_base` /
//! `GraphicsBase`): same mesh builders, same textures (reused here for
//! the physics bodies too — same cube/sphere/pyramid files as
//! `magic_figures`, plus the same floor texture), same lighting. No
//! bloom emissive glow here — these are ordinary lit, textured physics
//! props, not the "magic" glowing shapes.
//!
//! Run with:
//!   ./build.sh run physic_figures

use meridian_examples::{
    FlyCamera, GraphicsBase, cube_mesh_source, ground_mesh_source, icosphere_mesh_source,
    look_at_rotor, pyramid_mesh_source,
};
use meridian_gac_core::{Motor3, Vec3};
use meridian_graphics_core::{DrawBuffers, Light, Material, Renderable3D, Scene3D, submit_scene3d};
use meridian_graphics_driver::Device;
use meridian_physics_core::{
    BroadPhase, ColliderShape, ConstraintSolver, Integrator, NarrowPhase, RigidBody,
};
use meridian_platform_core::{AppHandler, InputState, KeyCode, Window, run_windowed_app};

const FLOOR_HALF_EXTENT: f32 = 14.0;
/// Floor collider is a thin static `Cuboid` slab, top surface at `y =
/// 0` (matching the rendered floor quad) — centering it at `-FLOOR_HALF_THICKNESS`
/// puts its top face exactly there.
const FLOOR_HALF_THICKNESS: f32 = 0.5;

const SPHERE_RADIUS: f32 = 0.6;
const CUBE_HALF_EXTENT: f32 = 0.6;
/// The pyramid's rendered mesh (see `pyramid_mesh_source`): base
/// half-extent and height below.
const PYRAMID_BASE_HALF_EXTENT: f32 = 0.65;
const PYRAMID_HEIGHT: f32 = 1.2;

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
/// How many times per physics tick the broad/narrow/solve sequence
/// re-runs against the same integrated positions — see
/// `PhysicsRig::step`'s doc comment for why one pass isn't enough for a
/// multi-point box manifold.
const SOLVER_RELAXATION_ITERATIONS: u32 = 4;

/// A `Cuboid` collider that roughly bounds the pyramid mesh (base
/// `2*PYRAMID_BASE_HALF_EXTENT` square, `PYRAMID_HEIGHT` tall) — see the
/// module doc for why this is a disclosed simplification rather than a
/// true pyramid collider. `RigidBody::frame`'s translation is the mesh's
/// own origin (the base center, per `pyramid_mesh_source`'s doc
/// comment), so the collider's vertical center must be offset up by
/// half its own height to bound the pyramid rather than being centered
/// on the ground plane the mesh sits on.
fn pyramid_collider_half_extents() -> Vec3 {
    Vec3::new(
        PYRAMID_BASE_HALF_EXTENT,
        PYRAMID_HEIGHT / 2.0,
        PYRAMID_BASE_HALF_EXTENT,
    )
}

struct GpuState {
    base: GraphicsBase,
    scene: Scene3D,
    /// Index into `scene.renderables` for each physics body, in the same
    /// order as `bodies` below.
    body_renderable_indices: [usize; 3],
}

struct PhysicsRig {
    bodies: Vec<RigidBody>,
    integrator: Integrator,
    solver: ConstraintSolver,
    broad: BroadPhase,
    narrow: NarrowPhase,
    /// Bodies `1..=3` are the sphere/cube/pyramid (body `0` is the
    /// static floor) — see [`PhysicsRig::new`].
    accumulator: f32,
}

impl PhysicsRig {
    fn new() -> Self {
        let floor = RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, -FLOOR_HALF_THICKNESS, 0.0)),
            mass: 0.0,
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(FLOOR_HALF_EXTENT, FLOOR_HALF_THICKNESS, FLOOR_HALF_EXTENT),
            },
            ..Default::default()
        };
        let sphere = RigidBody {
            frame: Motor3::translation(Vec3::new(-1.8, 4.0, 0.0)),
            mass: 1.0,
            shape: ColliderShape::Sphere {
                radius: SPHERE_RADIUS,
            },
            ..Default::default()
        };
        let cube = RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, 6.0, 0.0)),
            mass: 1.0,
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(CUBE_HALF_EXTENT, CUBE_HALF_EXTENT, CUBE_HALF_EXTENT),
            },
            ..Default::default()
        };
        let pyramid = RigidBody {
            frame: Motor3::translation(Vec3::new(1.8, 8.0, 0.0)),
            mass: 1.0,
            shape: ColliderShape::Cuboid {
                half_extents: pyramid_collider_half_extents(),
            },
            ..Default::default()
        };

        Self {
            bodies: vec![floor, sphere, cube, pyramid],
            integrator: Integrator::default(),
            solver: ConstraintSolver::new(SOLVER_RESTITUTION).with_friction(SOLVER_FRICTION),
            broad: BroadPhase::new(),
            narrow: NarrowPhase::new(),
            accumulator: 0.0,
        }
    }

    /// Fixed-timestep stepping (accumulator pattern): the render loop's
    /// `dt` varies with frame rate, but the solver is only validated at
    /// a constant `PHYSICS_DT` — accumulating and stepping in whole
    /// increments keeps the simulation deterministic regardless of how
    /// fast frames arrive, capped so a stall (e.g. window drag) can't
    /// spiral into running hundreds of catch-up steps at once.
    fn step(&mut self, frame_dt: f32) {
        self.accumulator += frame_dt;
        let mut steps = 0;
        while self.accumulator >= PHYSICS_DT && steps < 8 {
            self.integrator.step(&mut self.bodies, PHYSICS_DT);

            // Several *velocity-only* relaxation passes per tick, not
            // one: a box/pyramid manifold is up to 4 contact points
            // sharing one normal (see `NarrowPhase::generate_contacts`'s
            // box-box expansion), and one pass over all of them leaves
            // each point's impulse computed against the *other* points'
            // pre-solve velocity, which is why box/pyramid contacts kept
            // jittering on touchdown while the sphere (always exactly
            // one contact point) never did. Deliberately
            // `resolve_velocity`, not `resolve`: calling the *full*
            // `resolve` (which also applies positional correction) once
            // per relaxation pass pushed the body upward by the same
            // correction several times per tick, which is exactly the
            // "cube/pyramid bounce up/down and clip through the floor"
            // bug this split fixes — see `ConstraintSolver::resolve`'s
            // doc comment.
            for _ in 0..SOLVER_RELAXATION_ITERATIONS {
                let pairs = self.broad.find_candidate_pairs(&self.bodies).to_vec();
                for contact in self.narrow.generate_contacts(&self.bodies, &pairs) {
                    self.solver.resolve_velocity(&mut self.bodies, &contact);
                }
            }
            // Positional correction exactly once per tick, against the
            // final (velocity-relaxed) contact set.
            let pairs = self.broad.find_candidate_pairs(&self.bodies).to_vec();
            for contact in self.narrow.generate_contacts(&self.bodies, &pairs) {
                self.solver
                    .apply_positional_correction(&mut self.bodies, &contact);
            }

            self.accumulator -= PHYSICS_DT;
            steps += 1;
        }
    }
}

struct App {
    camera: FlyCamera,
    cursor_grabbed: bool,
    last_frame: std::time::Instant,
    physics: PhysicsRig,
    tokio_runtime: tokio::runtime::Runtime,
    gpu: Option<GpuState>,
}

impl App {
    fn new() -> Self {
        let tokio_runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        Self {
            camera: FlyCamera::new(Vec3::new(0.0, 3.0, 9.0)),
            cursor_grabbed: true,
            last_frame: std::time::Instant::now(),
            physics: PhysicsRig::new(),
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

        let floor_texture = base.load_texture("assets/textures/floor.png");
        let floor_material = base.materials.register(Material {
            albedo: Some(floor_texture),
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        });
        let floor_mesh = base
            .meshes
            .register(ground_mesh_source(FLOOR_HALF_EXTENT, 10.0))
            .expect("floor mesh must be valid");

        let sphere_texture = base.load_texture("assets/textures/sphere.png");
        let sphere_material = base.materials.register(Material {
            albedo: Some(sphere_texture),
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        });
        let sphere_mesh = base
            .meshes
            .register(icosphere_mesh_source(2, SPHERE_RADIUS))
            .expect("sphere mesh must be valid");

        let cube_texture = base.load_texture("assets/textures/cube.bmp");
        let cube_material = base.materials.register(Material {
            albedo: Some(cube_texture),
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        });
        let cube_mesh = base
            .meshes
            .register(cube_mesh_source(CUBE_HALF_EXTENT))
            .expect("cube mesh must be valid");

        let pyramid_texture = base.load_texture("assets/textures/pyramid.bmp");
        let pyramid_material = base.materials.register(Material {
            albedo: Some(pyramid_texture),
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        });
        // The pyramid mesh is centered on its own base (see
        // `pyramid_mesh_source`'s doc comment), but its physics collider
        // (a `Cuboid`) is centered on the body — so the mesh is rendered
        // shifted down by half the collider's height relative to the
        // body's frame, keeping the base flush with the collider's
        // bottom face.
        let pyramid_mesh = base
            .meshes
            .register(pyramid_mesh_source(
                PYRAMID_BASE_HALF_EXTENT,
                PYRAMID_HEIGHT,
            ))
            .expect("pyramid mesh must be valid");

        let renderables = vec![
            Renderable3D {
                mesh: floor_mesh,
                material: floor_material,
                frame: Motor3::identity(),
                billboard: false,
            },
            Renderable3D {
                mesh: sphere_mesh,
                material: sphere_material,
                frame: self.physics.bodies[1].frame,
                billboard: false,
            },
            Renderable3D {
                mesh: cube_mesh,
                material: cube_material,
                frame: self.physics.bodies[2].frame,
                billboard: false,
            },
            Renderable3D {
                mesh: pyramid_mesh,
                material: pyramid_material,
                frame: self.physics.bodies[3].frame,
                billboard: false,
            },
        ];

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
            ambient: [0.1, 0.1, 0.12],
            ..Scene3D::default()
        };

        self.gpu = Some(GpuState {
            base,
            scene,
            body_renderable_indices: [1, 2, 3],
        });
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
        for (renderable_index, body_index) in gpu.body_renderable_indices.iter().zip([1usize, 2, 3])
        {
            gpu.scene.renderables[*renderable_index].frame = self.physics.bodies[body_index].frame;
        }
        // The pyramid mesh's origin is its base center, but its
        // `Cuboid` collider is centered on the body — shift the
        // rendered frame down by the collider's half-height so the
        // mesh's base stays flush with the collider's resting contact
        // point instead of floating at the collider's vertical center.
        let pyramid_renderable = &mut gpu.scene.renderables[gpu.body_renderable_indices[2]];
        let drop = Motor3::translation(Vec3::new(0.0, -pyramid_collider_half_extents().y, 0.0));
        pyramid_renderable.frame = pyramid_renderable.frame.compose(drop);

        let aspect = window.width() as f32 / window.height().max(1) as f32;
        gpu.scene.camera = self.camera.camera(aspect);

        let frame = match gpu.base.surface.acquire_frame() {
            Ok(frame) => frame,
            Err(err) => {
                meridian_foundation::log_warn!(
                    "swapchain frame unavailable ({err}); reconfiguring surface"
                );
                gpu.base.resize(window.width(), window.height());
                return;
            }
        };

        let mut commands = gpu.base.device.create_command_buffer();
        let draw_buffers: Vec<DrawBuffers>;
        {
            let mut pass = commands.begin_render_pass(
                frame.view(),
                [0.05, 0.05, 0.08, 1.0],
                Some(&gpu.base.depth),
            );
            draw_buffers = submit_scene3d(
                &gpu.base.device,
                &gpu.base.renderer,
                &mut pass,
                &gpu.scene,
                &gpu.base.meshes,
                &gpu.base.materials,
                &gpu.base.textures,
            );
        }
        gpu.base.bloom.apply(
            &gpu.base.device,
            &mut commands,
            &gpu.base.renderer,
            &draw_buffers,
            &frame,
        );
        commands.submit();
        frame.present(&gpu.base.device);

        window.request_redraw();
    }

    fn on_resized(&mut self, width: u32, height: u32) {
        if let Some(gpu) = &mut self.gpu {
            gpu.base.resize(width, height);
        }
    }
}

fn main() {
    meridian_foundation::crash_reporting::install(meridian_foundation::CrashReportConfig::new(
        "physic_figures",
    ));
    meridian_foundation::logging::file::init(
        meridian_foundation::logging::file::FileLogConfig::new("physic_figures"),
    );
    run_windowed_app("Meridian Engine — Physic Figures", 1024, 768, App::new())
        .expect("windowed app exited with an error");
}
