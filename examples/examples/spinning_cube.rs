//! Roadmap milestone: the first real render-to-screen vertical slice —
//! `platform-core::Window` (real, `winit`-backed) -> a real windowed
//! `wgpu` swapchain surface (`graphics-driver::Device::new_windowed`) ->
//! a real render pipeline (vertex+fragment shading, depth-tested) -> a
//! spinning, lit cube on screen. `meridian-engine-core::Runtime` supplies
//! the frame clock (`Runtime::tick`'s `Time`) that drives the cube's
//! rotation, the same integration seam a real game loop would use — see
//! that crate's module doc for why `Runtime` itself doesn't gain
//! rendering awareness yet (no scene/material vocabulary or GPU-submission
//! bridge in `graphics-core` to justify it).
//!
//! Shadows are an explicit follow-up, not part of this pass (see
//! docs/roadmap.md).
//!
//! Run with:
//!   ./build.sh run spinning_cube

use meridian_engine_core::{Runtime, SubsystemManager};
use meridian_gac_core::{Motor3, Rotor, Vec3};
use meridian_gpu_driver::{BindGroup, Buffer};
use meridian_graphics_core::Camera;
use meridian_graphics_driver::{
    BufferUsage, DepthTexture, Device, RenderPipeline, Surface, VertexAttributeDesc, VertexFormat,
    VertexLayout,
};
use meridian_platform_core::{AppHandler, InputState, Window, run_windowed_app};

const SHADER: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.mvp * vec4<f32>(in.position, 1.0);
    let normal_matrix = mat3x3<f32>(u.model[0].xyz, u.model[1].xyz, u.model[2].xyz);
    out.world_normal = normalize(normal_matrix * in.normal);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.3));
    let diffuse = max(dot(in.world_normal, light_dir), 0.0);
    let ambient = 0.15;
    let base_color = vec3<f32>(0.85, 0.45, 0.2);
    let color = base_color * (ambient + diffuse * 0.85);
    return vec4<f32>(color, 1.0);
}
"#;

/// One cube vertex: position + a per-face normal (flat-shaded faces, not
/// smoothed — the standard way to make a cube's faces visually distinct
/// rather than looking like a rounded blob under lighting).
#[derive(Clone, Copy)]
struct CubeVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

/// 24 vertices (4 per face, not shared across faces — each face needs its
/// own normal) and 36 indices (6 faces * 2 triangles * 3), derived from
/// `u x v = face_normal` per face so every face's winding is consistently
/// counter-clockwise as seen from outside the cube, matching
/// `Device::create_render_pipeline`'s `FrontFace::Ccw` + back-face culling.
fn cube_mesh() -> (Vec<CubeVertex>, Vec<u16>) {
    fn face(normal: [f32; 3], corners: [[f32; 3]; 4]) -> [CubeVertex; 4] {
        corners.map(|position| CubeVertex { position, normal })
    }

    let faces = [
        face(
            [0.0, 0.0, 1.0],
            [
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
                [-1.0, 1.0, 1.0],
            ],
        ),
        face(
            [0.0, 0.0, -1.0],
            [
                [-1.0, -1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [1.0, 1.0, -1.0],
                [1.0, -1.0, -1.0],
            ],
        ),
        face(
            [1.0, 0.0, 0.0],
            [
                [1.0, -1.0, -1.0],
                [1.0, 1.0, -1.0],
                [1.0, 1.0, 1.0],
                [1.0, -1.0, 1.0],
            ],
        ),
        face(
            [-1.0, 0.0, 0.0],
            [
                [-1.0, -1.0, -1.0],
                [-1.0, -1.0, 1.0],
                [-1.0, 1.0, 1.0],
                [-1.0, 1.0, -1.0],
            ],
        ),
        face(
            [0.0, 1.0, 0.0],
            [
                [-1.0, 1.0, -1.0],
                [-1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, -1.0],
            ],
        ),
        face(
            [0.0, -1.0, 0.0],
            [
                [-1.0, -1.0, -1.0],
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, -1.0, -1.0],
            ],
        ),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for quad in faces {
        let base = vertices.len() as u16;
        vertices.extend_from_slice(&quad);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

fn vertices_to_bytes(vertices: &[CubeVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * 24);
    for v in vertices {
        for component in v.position.iter().chain(v.normal.iter()) {
            bytes.extend_from_slice(&component.to_le_bytes());
        }
    }
    bytes
}

fn indices_to_bytes(indices: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(indices.len() * 2);
    for i in indices {
        bytes.extend_from_slice(&i.to_le_bytes());
    }
    bytes
}

fn mat4_to_bytes(m: [[f32; 4]; 4]) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    let mut offset = 0;
    for column in m {
        for component in column {
            bytes[offset..offset + 4].copy_from_slice(&component.to_le_bytes());
            offset += 4;
        }
    }
    bytes
}

/// A camera rotor that turns `gac-core`'s local-forward `+X` toward
/// `target - eye` via a single axis/angle rotation (no roll control, but
/// this demo doesn't need one). See `graphics-core`'s `Camera` doc
/// comment for the local-forward convention this aligns to.
fn look_at_rotor(eye: Vec3, target: Vec3) -> Rotor {
    let forward = (target - eye).normalize();
    let local_forward = Vec3::X;
    let cos_angle = local_forward.dot(forward).clamp(-1.0, 1.0);
    if cos_angle > 0.9999 {
        return Rotor::identity();
    }
    if cos_angle < -0.9999 {
        return Rotor::from_axis_angle(Vec3::Y, core::f32::consts::PI);
    }
    let axis = local_forward.cross(forward).normalize();
    Rotor::from_axis_angle(axis, cos_angle.acos())
}

/// GPU resources built once the OS window exists — see
/// `App::on_ready`.
struct GpuState {
    device: Device,
    surface: Surface,
    depth: DepthTexture,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
}

struct App {
    tokio_runtime: tokio::runtime::Runtime,
    engine_runtime: Runtime,
    gpu: Option<GpuState>,
}

impl App {
    fn new() -> Self {
        Self {
            tokio_runtime: tokio::runtime::Runtime::new().expect("failed to start tokio runtime"),
            engine_runtime: Runtime::new(SubsystemManager::new(meridian_audio_core::Mixer::new(
                meridian_audio_core::SpeakerLayout::stereo_headphones(),
            ))),
            gpu: None,
        }
    }
}

impl AppHandler for App {
    fn on_ready(&mut self, window: &Window) {
        let target = window.surface_target();
        let (width, height) = (window.width(), window.height());

        // Device::new_windowed is a real async fn (an OS/driver handshake
        // to open the GPU, the same class of operation as Device::new —
        // see graphics-driver's module doc). winit's ApplicationHandler
        // callbacks aren't async, so this is the one place the example
        // bridges into the tokio runtime it owns.
        let (device, surface) = self
            .tokio_runtime
            .block_on(Device::new_windowed(target, width, height))
            .expect("failed to create windowed GPU device");

        let depth = device.create_depth_texture(width, height);
        let shader = device.create_shader("spinning_cube", SHADER);
        let vertex_layout = VertexLayout {
            stride: 24, // 6 x f32
            attributes: vec![
                VertexAttributeDesc {
                    location: 0,
                    format: VertexFormat::Float32x3,
                    offset: 0,
                },
                VertexAttributeDesc {
                    location: 1,
                    format: VertexFormat::Float32x3,
                    offset: 12,
                },
            ],
        };
        let pipeline = device.create_render_pipeline(
            &shader,
            "vs_main",
            "fs_main",
            &vertex_layout,
            &surface,
            true,
        );

        let (vertices, indices) = cube_mesh();
        let vertex_bytes = vertices_to_bytes(&vertices);
        let vertex_buffer = device.create_buffer(vertex_bytes.len(), BufferUsage::Vertex);
        device.write_buffer(&vertex_buffer, &vertex_bytes);

        let index_bytes = indices_to_bytes(&indices);
        let index_buffer = device.create_buffer(index_bytes.len(), BufferUsage::Index);
        device.write_buffer(&index_buffer, &index_bytes);

        let uniform_buffer = device.create_buffer(128, BufferUsage::Uniform);
        let bind_group = device.create_uniform_bind_group(&pipeline, &uniform_buffer);

        self.gpu = Some(GpuState {
            device,
            surface,
            depth,
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            uniform_buffer,
            bind_group,
        });
    }

    fn on_redraw(&mut self, window: &Window, _input: &InputState) {
        let Some(gpu) = &mut self.gpu else {
            return;
        };
        let time = self.engine_runtime.tick();

        let camera = Camera {
            frame: Motor3::from_rotation_translation(
                look_at_rotor(Vec3::new(4.0, 3.0, 5.0), Vec3::ZERO),
                Vec3::new(4.0, 3.0, 5.0),
            ),
            projection: meridian_gac_core::Projection::perspective(
                60.0_f32.to_radians(),
                window.width() as f32 / window.height().max(1) as f32,
                0.1,
                100.0,
            ),
        };

        let spin = time.total_seconds as f32;
        let model = Motor3::rotation(Vec3::new(0.3, 1.0, 0.15).normalize(), spin);
        let mvp =
            meridian_gac_core::float_ga::mat4_mul(camera.view_projection_matrix(), model.to_mat4());

        let mut uniform_bytes = Vec::with_capacity(128);
        uniform_bytes.extend_from_slice(&mat4_to_bytes(mvp));
        uniform_bytes.extend_from_slice(&mat4_to_bytes(model.to_mat4()));
        gpu.device.write_buffer(&gpu.uniform_buffer, &uniform_bytes);

        let frame = match gpu.surface.acquire_frame() {
            Ok(frame) => frame,
            Err(err) => {
                // A lost/outdated swapchain (another GPU client appearing,
                // a display change) must be reconfigured, not just skipped
                // — skipping forever is the "silent gray window" failure.
                meridian_foundation::log_warn!(
                    "swapchain frame unavailable ({err}); reconfiguring surface"
                );
                gpu.surface
                    .resize(&gpu.device, window.width(), window.height());
                return;
            }
        };

        let mut commands = gpu.device.create_command_buffer();
        {
            let mut pass =
                commands.begin_render_pass(frame.view(), [0.05, 0.05, 0.08, 1.0], Some(&gpu.depth));
            pass.set_pipeline(&gpu.pipeline);
            pass.set_bind_group(0, &gpu.bind_group);
            pass.set_vertex_buffer(0, &gpu.vertex_buffer);
            pass.set_index_buffer_u16(&gpu.index_buffer);
            pass.draw_indexed(0..gpu.index_count);
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
    meridian_foundation::crash_reporting::install(meridian_foundation::CrashReportConfig::new(
        "spinning_cube",
    ));
    meridian_foundation::logging::file::init(
        meridian_foundation::logging::file::FileLogConfig::new("spinning_cube"),
    );
    run_windowed_app("Meridian Engine — Spinning Cube", 960, 720, App::new())
        .expect("windowed app exited with an error");
}
