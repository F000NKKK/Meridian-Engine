//! Same scene as `soft_body_jiggle_deterministic` — many small soft
//! bodies jiggling in place, split 50/50 between GPU
//! (`meridian_physics_compute::float::SoftBodyGpuKernel`) and CPU
//! (`SoftBodyIntegrator::step`) — but `f32`, not `Fixed`. The explicit
//! point of comparison: there is **no** bit-exact determinism claim
//! here, and this example doesn't try to prove one (unlike
//! `soft_body_jiggle_deterministic`'s shadow-pair `assert_eq!`). GPU
//! float summation order can differ from the CPU's, so a GPU-stepped
//! ball and a CPU-stepped ball given identical initial conditions will
//! visibly drift apart over enough frames (usually imperceptibly at
//! first, then — because a mass-spring system is chaotic once collision/
//! feedback are in play — potentially a lot, the same sensitivity
//! `meridian-physics-compute::float`'s own test documents). Watching
//! this example next to the deterministic one side by side is the
//! demonstration: both look similar at a glance, only one of them is
//! actually reproducible.
//!
//! Camera is a free-fly `meridian_examples::FlyCamera` (WASD + hold right
//! mouse button to look, Space/Ctrl for up/down, Shift to move faster).
//!
//! Run with:
//!   ./build.sh run soft_body_jiggle_float

use meridian_examples::{
    FlyCamera, GROUND_SHADER, SOFT_BODY_SHADER, ground_quad_buffers, mat4_to_bytes,
    soft_body_render_buffers, soft_body_vertex_layout,
};
use meridian_gac_core::generic::Plane;
use meridian_gac_core::{Vec3, icosphere};
use meridian_gpu_driver::{BindGroup, Buffer};
use meridian_graphics_driver::{BufferUsage, DepthTexture, Device, RenderPipeline, Surface};
use meridian_physics_compute::float::SoftBodyGpuKernel;
use meridian_physics_core::soft_body::float_softbody::{
    SoftBody, SoftBodyIntegrator, icosphere_soft_body,
};
use meridian_platform_core::{AppHandler, InputState, Window, run_windowed_app};

const PHYSICS_DT: f32 = 1.0 / 240.0;
const MAX_SUBSTEPS_PER_FRAME: u32 = 8;
const BALL_COUNT: usize = 8;

enum Backend {
    Gpu,
    Cpu,
}

struct Ball {
    body: SoftBody,
    backend: Backend,
}

/// See `soft_body_jiggle_deterministic::spawn_ball` — identical
/// construction, `f32` instead of `Fixed`.
fn spawn_ball(center: Vec3) -> SoftBody {
    let mut body = icosphere_soft_body(center, 0.35, 1, 0.05, 400.0, 2.0, 150.0, 1.0);
    let center_index = body.particle_count() - 1;
    body.inverse_masses[center_index] = 0.0;
    let impulse = 0.4;
    for i in 0..center_index {
        let direction = (body.positions[i] - body.positions[center_index]).normalize();
        body.velocities[i] = direction * impulse;
    }
    body
}

fn spawn_balls() -> Vec<Ball> {
    let mut balls = Vec::with_capacity(BALL_COUNT);
    for i in 0..BALL_COUNT {
        let row = (i / 4) as f32;
        let col = (i % 4) as f32;
        let center = Vec3::new(col - 1.5, row + 1.0, 0.0);
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
    ground_pipeline: RenderPipeline,
    ground_bind_group: BindGroup,
    ground_vertex_buffer: Buffer,
    ground_index_buffer: Buffer,
    ground_index_count: u32,
}

struct App {
    tokio_runtime: tokio::runtime::Runtime,
    compute_context: meridian_compute_runtime::ComputeContext,
    kernel: SoftBodyGpuKernel,
    integrator: SoftBodyIntegrator,
    balls: Vec<Ball>,
    accumulator: f32,
    last_frame: std::time::Instant,
    faces: Vec<meridian_gac_core::generic::Face>,
    gpu: Option<GpuState>,
}

impl App {
    fn new() -> Self {
        let tokio_runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        let compute_context = tokio_runtime
            .block_on(meridian_compute_runtime::ComputeContext::new().with_gpu())
            .expect("this example needs a real GPU compute backend");
        let kernel = SoftBodyGpuKernel::new(&compute_context);
        let integrator = SoftBodyIntegrator::new(
            Vec3::ZERO,
            Plane {
                normal: Vec3::Y,
                d: 100.0,
            },
            0.3,
        );
        Self {
            tokio_runtime,
            compute_context,
            kernel,
            integrator,
            balls: spawn_balls(),
            accumulator: 0.0,
            last_frame: std::time::Instant::now(),
            faces: icosphere(1).faces,
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
        let shader = device.create_shader("soft_body_jiggle_float", SOFT_BODY_SHADER);
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

        // Purely a visual height reference — see
        // `soft_body_jiggle_deterministic`'s identical comment.
        let ground_shader = device.create_shader("ground", GROUND_SHADER);
        let ground_pipeline = device.create_render_pipeline(
            &ground_shader,
            "vs_main",
            "fs_main",
            &soft_body_vertex_layout(),
            &surface,
            true,
        );
        let ground_bind_group = device.create_uniform_bind_group(&ground_pipeline, &uniform_buffer);
        let (ground_vertex_bytes, ground_index_bytes, ground_index_count) =
            ground_quad_buffers(30.0, 0.0);
        let ground_vertex_buffer =
            device.create_buffer(ground_vertex_bytes.len(), BufferUsage::Vertex);
        device.write_buffer(&ground_vertex_buffer, &ground_vertex_bytes);
        let ground_index_buffer =
            device.create_buffer(ground_index_bytes.len(), BufferUsage::Index);
        device.write_buffer(&ground_index_buffer, &ground_index_bytes);

        self.gpu = Some(GpuState {
            device,
            surface,
            depth,
            pipeline,
            uniform_buffer,
            bind_group,
            ground_pipeline,
            ground_bind_group,
            ground_vertex_buffer,
            ground_index_buffer,
            ground_index_count,
        });
    }

    fn on_redraw(&mut self, window: &Window, _input: &InputState) {
        let Some(gpu) = &self.gpu else {
            return;
        };

        let now = std::time::Instant::now();
        let frame_dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.accumulator += frame_dt;

        let mut substeps = 0;
        while self.accumulator >= PHYSICS_DT && substeps < MAX_SUBSTEPS_PER_FRAME {
            for ball in &mut self.balls {
                match ball.backend {
                    Backend::Gpu => {
                        self.tokio_runtime.block_on(self.kernel.step(
                            &self.compute_context,
                            &mut ball.body,
                            self.integrator.gravity,
                            self.integrator.ground,
                            self.integrator.restitution,
                            PHYSICS_DT,
                        ));
                    }
                    Backend::Cpu => {
                        self.integrator.step(&mut ball.body, PHYSICS_DT);
                    }
                }
            }
            self.accumulator -= PHYSICS_DT;
            substeps += 1;
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
                let surface_positions = &ball.body.positions[0..surface_count];
                let (vertex_bytes, index_bytes, index_count) =
                    soft_body_render_buffers(surface_positions, &self.faces);

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

            pass.set_pipeline(&gpu.ground_pipeline);
            pass.set_bind_group(0, &gpu.ground_bind_group);
            pass.set_vertex_buffer(0, &gpu.ground_vertex_buffer);
            pass.set_index_buffer_u16(&gpu.ground_index_buffer);
            pass.draw_indexed(0..gpu.ground_index_count);

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
        "Meridian Engine — Soft-Body Jiggle (float, GPU/CPU 50/50, non-deterministic)",
        1024,
        768,
        App::new(),
    )
    .expect("windowed app exited with an error");
}
