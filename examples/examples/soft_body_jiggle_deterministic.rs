//! Many small `Fixed`-point soft bodies jiggling in place (no gravity —
//! each one is a pinned-center ball given a small initial radial
//! velocity impulse, so surface springs pull it back and it oscillates),
//! split 50/50 between GPU (`meridian_physics_compute::fixed::FixedSoftBodyGpuKernel`)
//! and CPU (`FixedSoftBodyIntegrator::step`) — proving the split doesn't
//! matter for determinism, not just running two backends side by side
//! for show.
//!
//! The actual proof: alongside the rendered population, one ball
//! (`shadow`) is stepped through *both* backends every frame from
//! identical starting state, and its GPU/CPU results are asserted
//! bit-exact (`assert_eq!` on the raw `Fixed` positions/velocities) —
//! the live version of `meridian-physics-compute::fixed`'s own
//! `gpu_step_matches_cpu_step_bit_exact` test, run continuously instead
//! of for a fixed number of steps. A mismatch panics immediately rather
//! than silently rendering wrong output — this example *is* the
//! integration test for cross-backend determinism, not just a demo of
//! it.
//!
//! Run with:
//!   ./build.sh run soft_body_jiggle_deterministic

use meridian_examples::{
    SOFT_BODY_SHADER, look_at_rotor, mat4_to_bytes, soft_body_render_buffers,
    soft_body_vertex_layout,
};
use meridian_gac_core::fixed_ga::{FixedVec3, fixed_icosphere};
use meridian_gac_core::generic::Plane;
use meridian_gac_core::{Motor3, Vec3};
use meridian_gpu_driver::{BindGroup, Buffer};
use meridian_graphics_core::Camera;
use meridian_graphics_driver::{BufferUsage, DepthTexture, Device, RenderPipeline, Surface};
use meridian_numeric_core::Fixed;
use meridian_physics_compute::fixed::FixedSoftBodyGpuKernel;
use meridian_physics_core::soft_body::fixed_softbody::{
    FixedSoftBody, FixedSoftBodyIntegrator, fixed_icosphere_soft_body,
};
use meridian_platform_core::{AppHandler, InputState, Window, run_windowed_app};

const PHYSICS_DT_SECONDS: f64 = 1.0 / 240.0;
const MAX_SUBSTEPS_PER_FRAME: u32 = 8;
const BALL_COUNT: usize = 8;

enum Backend {
    Gpu,
    Cpu,
}

struct Ball {
    body: FixedSoftBody,
    backend: Backend,
}

fn fv3(x: f64, y: f64, z: f64) -> FixedVec3 {
    FixedVec3::new(Fixed::from_num(x), Fixed::from_num(y), Fixed::from_num(z))
}

/// Builds one small ball at `center`, its center particle pinned
/// (`inverse_mass = 0`) and a *single* surface particle given a small
/// outward velocity impulse — the "pluck" that starts it jiggling, since
/// a soft body at rest with zero gravity would otherwise just sit
/// motionless. Deliberately not a simultaneous push on every surface
/// particle: that's a much harsher transient (every spoke spring
/// stretches at once instead of the disturbance propagating outward
/// through the mesh) and overflowed `Fixed` even with `physics-core`'s
/// own proven-stable stiffness/mass/damping below.
fn spawn_ball(center: FixedVec3) -> FixedSoftBody {
    // Same stiffness/mass/damping `physics-core::soft_body::fixed_softbody`'s
    // own tests proved stable at `dt = 1/240` (see that module's
    // `identical_inputs_produce_bit_identical_output_after_many_steps`
    // comment on why: explicit-Euler mass-spring integration is only
    // conditionally stable, `omega * dt` has to stay small).
    let mut body = fixed_icosphere_soft_body(
        center,
        Fixed::from_num(0.35),
        1,
        Fixed::from_num(0.05),
        Fixed::from_num(400.0),
        Fixed::from_num(2.0),
        Fixed::from_num(150.0),
        Fixed::from_num(1.0),
    );
    let center_index = body.particle_count() - 1;
    body.inverse_masses[center_index] = Fixed::ZERO;
    let direction = (body.positions[0] - body.positions[center_index]).normalize();
    body.velocities[0] = direction * Fixed::from_num(0.6);
    body
}

fn spawn_balls() -> Vec<Ball> {
    let mut balls = Vec::with_capacity(BALL_COUNT);
    for i in 0..BALL_COUNT {
        let row = (i / 4) as f64;
        let col = (i % 4) as f64;
        let center = fv3(col * 1.0 - 1.5, row * 1.0 + 1.0, 0.0);
        let backend = if i % 2 == 0 {
            Backend::Gpu
        } else {
            Backend::Cpu
        };
        balls.push(Ball {
            body: spawn_ball(center),
            backend,
        });
    }
    balls
}

struct GpuState {
    device: Device,
    surface: Surface,
    depth: DepthTexture,
    pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
}

struct App {
    tokio_runtime: tokio::runtime::Runtime,
    compute_context: meridian_compute_runtime::ComputeContext,
    kernel: FixedSoftBodyGpuKernel,
    integrator: FixedSoftBodyIntegrator,
    balls: Vec<Ball>,
    shadow_gpu: FixedSoftBody,
    shadow_cpu: FixedSoftBody,
    accumulator_seconds: f64,
    last_frame: std::time::Instant,
    faces: Vec<meridian_gac_core::generic::Face>,
    frame_counter: u64,
    gpu: Option<GpuState>,
}

impl App {
    fn new() -> Self {
        let tokio_runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        let compute_context = tokio_runtime
            .block_on(meridian_compute_runtime::ComputeContext::new().with_gpu())
            .expect("this example needs a real GPU compute backend");
        let kernel = FixedSoftBodyGpuKernel::new(&compute_context);
        let integrator = FixedSoftBodyIntegrator::new(
            FixedVec3::ZERO,
            Plane {
                normal: fv3(0.0, 1.0, 0.0),
                d: Fixed::from_num(-100.0),
            },
            Fixed::from_num(0.3),
        );
        let shadow_seed = spawn_ball(fv3(3.5, 1.0, 0.0));
        Self {
            tokio_runtime,
            compute_context,
            kernel,
            integrator,
            balls: spawn_balls(),
            shadow_gpu: shadow_seed.clone(),
            shadow_cpu: shadow_seed,
            accumulator_seconds: 0.0,
            last_frame: std::time::Instant::now(),
            faces: fixed_icosphere(1).faces,
            frame_counter: 0,
            gpu: None,
        }
    }
}

impl AppHandler for App {
    fn on_ready(&mut self, window: &Window) {
        let target = window.surface_target();
        let (width, height) = (window.width(), window.height());
        let (device, surface) = self
            .tokio_runtime
            .block_on(Device::new_windowed(target, width, height))
            .expect("failed to create windowed GPU device");

        let depth = device.create_depth_texture(width, height);
        let shader = device.create_shader("soft_body_jiggle_deterministic", SOFT_BODY_SHADER);
        let pipeline = device.create_render_pipeline(
            &shader,
            "vs_main",
            "fs_main",
            &soft_body_vertex_layout(),
            &surface,
            true,
        );
        let uniform_buffer = device.create_buffer(64, BufferUsage::Uniform);
        let bind_group = device.create_uniform_bind_group(&pipeline, &uniform_buffer);

        self.gpu = Some(GpuState {
            device,
            surface,
            depth,
            pipeline,
            uniform_buffer,
            bind_group,
        });
    }

    fn on_redraw(&mut self, window: &Window, _input: &InputState) {
        let Some(gpu) = &self.gpu else {
            return;
        };

        let now = std::time::Instant::now();
        let frame_dt = (now - self.last_frame).as_secs_f64().min(0.1);
        self.last_frame = now;
        self.accumulator_seconds += frame_dt;
        let dt = Fixed::from_num(PHYSICS_DT_SECONDS);

        let mut substeps = 0;
        while self.accumulator_seconds >= PHYSICS_DT_SECONDS && substeps < MAX_SUBSTEPS_PER_FRAME {
            for ball in &mut self.balls {
                match ball.backend {
                    Backend::Gpu => {
                        self.tokio_runtime.block_on(self.kernel.step(
                            &self.compute_context,
                            &mut ball.body,
                            self.integrator.gravity,
                            self.integrator.ground,
                            self.integrator.restitution,
                            dt,
                        ));
                    }
                    Backend::Cpu => {
                        self.integrator.step(&mut ball.body, dt);
                    }
                }
            }

            // The determinism proof: step the shadow pair through both
            // backends from the same prior state and require bit-exact
            // agreement — see the module doc.
            self.tokio_runtime.block_on(self.kernel.step(
                &self.compute_context,
                &mut self.shadow_gpu,
                self.integrator.gravity,
                self.integrator.ground,
                self.integrator.restitution,
                dt,
            ));
            self.integrator.step(&mut self.shadow_cpu, dt);
            assert_eq!(
                self.shadow_gpu.positions, self.shadow_cpu.positions,
                "GPU/CPU determinism broken: positions diverged at frame {}",
                self.frame_counter
            );
            assert_eq!(
                self.shadow_gpu.velocities, self.shadow_cpu.velocities,
                "GPU/CPU determinism broken: velocities diverged at frame {}",
                self.frame_counter
            );

            self.accumulator_seconds -= PHYSICS_DT_SECONDS;
            substeps += 1;
            self.frame_counter += 1;
            if self.frame_counter % 240 == 0 {
                println!(
                    "[frame {}] GPU/CPU determinism check: OK ({BALL_COUNT} balls, 50/50 split)",
                    self.frame_counter
                );
            }
        }

        let camera = Camera {
            frame: Motor3::from_rotation_translation(
                look_at_rotor(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 1.5, 0.0)),
                Vec3::new(0.0, 2.0, 6.0),
            ),
            projection: meridian_gac_core::Projection::perspective(
                55.0_f32.to_radians(),
                window.width() as f32 / window.height().max(1) as f32,
                0.1,
                100.0,
            ),
        };
        gpu.device.write_buffer(
            &gpu.uniform_buffer,
            &mat4_to_bytes(camera.view_projection_matrix()),
        );

        let frame = match gpu.surface.acquire_frame() {
            Ok(frame) => frame,
            Err(err) => {
                eprintln!("skipping frame: {err}");
                return;
            }
        };

        let surface_count = self
            .faces
            .iter()
            .flat_map(|f| f.indices.iter())
            .max()
            .map(|&i| i + 1)
            .unwrap_or(0);
        let ball_buffers: Vec<(Buffer, Buffer, u32)> = self
            .balls
            .iter()
            .map(|ball| {
                let surface_positions_fixed = &ball.body.positions[0..surface_count];
                let surface_positions: Vec<Vec3> = surface_positions_fixed
                    .iter()
                    .map(|p| {
                        Vec3::new(
                            p.x.to_num() as f32,
                            p.y.to_num() as f32,
                            p.z.to_num() as f32,
                        )
                    })
                    .collect();
                let (vertex_bytes, index_bytes, index_count) =
                    soft_body_render_buffers(&surface_positions, &self.faces);

                let vertex_buffer = gpu
                    .device
                    .create_buffer(vertex_bytes.len(), BufferUsage::Vertex);
                gpu.device.write_buffer(&vertex_buffer, &vertex_bytes);
                let index_buffer = gpu
                    .device
                    .create_buffer(index_bytes.len(), BufferUsage::Index);
                gpu.device.write_buffer(&index_buffer, &index_bytes);

                (vertex_buffer, index_buffer, index_count)
            })
            .collect();

        let mut commands = gpu.device.create_command_buffer();
        {
            let mut pass =
                commands.begin_render_pass(frame.view(), [0.05, 0.05, 0.08, 1.0], Some(&gpu.depth));
            pass.set_pipeline(&gpu.pipeline);
            pass.set_bind_group(0, &gpu.bind_group);

            for (vertex_buffer, index_buffer, index_count) in &ball_buffers {
                pass.set_vertex_buffer(0, vertex_buffer);
                pass.set_index_buffer_u16(index_buffer);
                pass.draw_indexed(0..*index_count);
            }
        }
        commands.submit();
        frame.present(&gpu.device);

        window.request_redraw();
    }

    fn on_resized(&mut self, width: u32, height: u32) {
        if let Some(gpu) = &mut self.gpu {
            gpu.surface.resize(&gpu.device, width, height);
            gpu.depth = gpu.device.create_depth_texture(width, height);
        }
    }
}

fn main() {
    run_windowed_app(
        "Meridian Engine — Deterministic Soft-Body Jiggle (GPU/CPU 50/50)",
        1024,
        768,
        App::new(),
    )
    .expect("windowed app exited with an error");
}
