//! Real soft-body physics, GPU-computed: several rubber balls (mass-spring
//! `SoftBody`, `physics-core::soft_body::float_softbody`) drop onto the
//! ground and visibly deform on impact, then recover their shape — the
//! same behavior `float_softbody::ball_deforms_on_impact_and_recovers_its_shape`
//! proves on the CPU, except every step here runs through
//! `meridian_physics_compute::float::SoftBodyGpuKernel` (a real WGSL
//! compute dispatch per ball per frame), not `SoftBodyIntegrator::step`.
//!
//! Windowing/render scaffolding mirrors `spinning_cube`; the
//! mesh-topology-to-vertex-buffer conversion is shared with the other two
//! soft-body examples via `meridian_examples` (see that crate's module
//! doc).
//!
//! Run with:
//!   ./build.sh run soft_body_rubber_balls

use meridian_examples::{
    SOFT_BODY_SHADER, look_at_rotor, mat4_to_bytes, soft_body_render_buffers,
    soft_body_vertex_layout,
};
use meridian_gac_core::generic::Plane;
use meridian_gac_core::{Motor3, Vec3, icosphere};
use meridian_gpu_driver::{BindGroup, Buffer};
use meridian_graphics_core::Camera;
use meridian_graphics_driver::{BufferUsage, DepthTexture, Device, RenderPipeline, Surface};
use meridian_physics_compute::float::SoftBodyGpuKernel;
use meridian_physics_core::soft_body::float_softbody::{
    SoftBody, SoftBodyIntegrator, icosphere_soft_body,
};
use meridian_platform_core::{AppHandler, InputState, Window, run_windowed_app};

/// Fixed physics timestep — matches `float_softbody`'s own stability
/// note (`dt = 1/240`, not `1/60`; explicit-Euler mass-spring
/// integration is only conditionally stable, see that module's doc).
const PHYSICS_DT: f32 = 1.0 / 240.0;
const MAX_SUBSTEPS_PER_FRAME: u32 = 8;

struct Ball {
    body: SoftBody,
}

fn spawn_balls() -> Vec<Ball> {
    let centers = [
        Vec3::new(-1.2, 2.5, 0.0),
        Vec3::new(0.0, 3.5, 0.3),
        Vec3::new(1.3, 3.0, -0.2),
    ];
    centers
        .into_iter()
        .map(|center| Ball {
            body: icosphere_soft_body(center, 0.5, 1, 0.05, 400.0, 2.0, 150.0, 1.0),
        })
        .collect()
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
            Vec3::new(0.0, -9.81, 0.0),
            Plane {
                normal: Vec3::Y,
                d: 0.0,
            },
            0.25,
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
        let shader = device.create_shader("soft_body_rubber_balls", SOFT_BODY_SHADER);
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
        let frame_dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.accumulator += frame_dt;

        let mut substeps = 0;
        while self.accumulator >= PHYSICS_DT && substeps < MAX_SUBSTEPS_PER_FRAME {
            for ball in &mut self.balls {
                self.tokio_runtime.block_on(self.kernel.step(
                    &self.compute_context,
                    &mut ball.body,
                    self.integrator.gravity,
                    self.integrator.ground,
                    self.integrator.restitution,
                    PHYSICS_DT,
                ));
            }
            self.accumulator -= PHYSICS_DT;
            substeps += 1;
        }

        let camera = Camera {
            frame: Motor3::from_rotation_translation(
                look_at_rotor(Vec3::new(0.0, 2.5, 6.0), Vec3::new(0.0, 1.0, 0.0)),
                Vec3::new(0.0, 2.5, 6.0),
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

        // Built up front and kept alive through `commands.submit()` below
        // — a buffer dropped while the render pass still references it
        // (e.g. if these were built inline inside the pass's loop) would
        // invalidate the recorded draw call before the GPU ever sees it.
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
        "Meridian Engine — Soft-Body Rubber Balls (GPU)",
        1024,
        768,
        App::new(),
    )
    .expect("windowed app exited with an error");
}
