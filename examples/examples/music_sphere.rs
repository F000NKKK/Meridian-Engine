//! A sphere hanging in space, emitting the background music from
//! `examples/assets/audio/` (decoded by signature via
//! `asset-core::AnyAudioDecoder` — MP3/Opus/Vorbis/FLAC/WAV all work,
//! see ADR 013), and a free-fly camera whose pose *is* the audio
//! listener: fly around the sphere and the music tracks your head
//! through `audio-core`'s `BinauralRenderer` — interaural time
//! difference (the far ear hears ~0.6 ms later), head-shadow filtering
//! (the far ear loses high frequencies, lows wrap around), rear damping
//! (quieter and duller behind you) and distance attenuation, all with
//! per-block parameter ramps so a moving camera never crackles.
//! `FlyCamera`'s `Motor3` frame uses the same local-forward-`+X`
//! convention as `audio-core`'s `Listener`, so the camera frame is
//! handed to the renderer unchanged.
//!
//! Audio is fed without ever blocking the render thread: each frame
//! tops the stream's ring buffer up with `can_push`-guarded ~50 ms
//! blocks, mixed against the listener's *current* pose, so panning
//! tracks the camera with at most one ring buffer of latency. The track
//! loops.
//!
//! Controls as in the soft-body examples: WASD + mouse (cursor grabbed
//! on launch), Space/Ctrl up/down, Shift faster, Escape toggles the
//! cursor grab.
//!
//! Run with:
//!   ./build.sh run music_sphere

use meridian_asset_core::{AnyAudioDecoder, Decoder};
use meridian_audio_core::{AudioOutput, BinauralRenderer, Emitter, Listener, SpeakerLayout};
use meridian_examples::{
    FlyCamera, GROUND_SHADER, SOFT_BODY_SHADER, ground_quad_buffers, mat4_to_bytes,
    soft_body_render_buffers, soft_body_vertex_layout,
};
use meridian_gac_core::{Motor3, Vec3, icosphere};
use meridian_gpu_driver::{BindGroup, Buffer};
use meridian_graphics_driver::{BufferUsage, DepthTexture, Device, RenderPipeline, Surface};
use meridian_platform_core::{AppHandler, InputState, KeyCode, Window, run_windowed_app};

const SPHERE_CENTER: Vec3 = Vec3 {
    x: 0.0,
    y: 1.5,
    z: 0.0,
};
const SPHERE_RADIUS: f32 = 0.75;
/// ~50 ms of audio per mixed block.
const CHUNK_SECONDS: f32 = 0.05;

/// The looping music track, mixed spatially against the current
/// listener pose and topped up into the output stream without blocking.
struct MusicSource {
    output: AudioOutput,
    renderer: BinauralRenderer,
    /// The decoded track, downmixed to mono — the emitter is a point
    /// source; its spatialization *is* the stereo image.
    mono: Vec<f32>,
    cursor: usize,
    chunk_frames: usize,
}

impl MusicSource {
    /// Decodes the first playable file in `examples/assets/audio/` (by
    /// signature, not extension) and opens a stereo output at the
    /// track's own sample rate.
    async fn load() -> Result<Self, String> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/audio");
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| format!("cannot read {dir}: {e}"))?
            .flatten()
            .map(|e| e.path())
            .collect();
        entries.sort();

        for path in entries {
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Some(format) = AnyAudioDecoder.sniff(&bytes) else {
                continue;
            };
            let audio = match AnyAudioDecoder.decode(&bytes) {
                Ok(audio) => audio,
                Err(err) => {
                    eprintln!("  {}: {err}", path.display());
                    continue;
                }
            };
            println!(
                "playing {} ({format:?}, {} Hz, {} ch)",
                path.display(),
                audio.sample_rate,
                audio.channels
            );

            let channels = audio.channels.max(1) as usize;
            let mono: Vec<f32> = audio
                .samples
                .chunks_exact(channels)
                .map(|frame| {
                    frame.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32
                })
                .collect();

            let renderer = BinauralRenderer::new(audio.sample_rate);
            let output = AudioOutput::open(&SpeakerLayout::stereo_headphones(), audio.sample_rate)
                .await
                .map_err(|e| e.to_string())?;
            return Ok(Self {
                output,
                renderer,
                mono,
                cursor: 0,
                chunk_frames: (audio.sample_rate as f32 * CHUNK_SECONDS) as usize,
            });
        }
        Err(format!("no decodable audio file in {dir}"))
    }

    /// Tops the output ring buffer up with blocks mixed against
    /// `listener`'s current pose. Never blocks: pushes only while the
    /// stream reports room for a whole block.
    fn refill(&mut self, listener: &Listener) {
        let emitter = Emitter {
            frame: Motor3::translation(SPHERE_CENTER),
        };
        while self.output.can_push(self.chunk_frames * 2) {
            let mut chunk = Vec::with_capacity(self.chunk_frames);
            for _ in 0..self.chunk_frames {
                chunk.push(self.mono[self.cursor]);
                self.cursor = (self.cursor + 1) % self.mono.len(); // loop the track
            }
            let interleaved =
                self.renderer
                    .render(listener, &[(emitter, &chunk)], self.chunk_frames);
            self.output.push_interleaved(&interleaved);
        }
    }
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
    sphere_vertex_buffer: Buffer,
    sphere_index_buffer: Buffer,
    sphere_index_count: u32,
}

struct App {
    tokio_runtime: tokio::runtime::Runtime,
    camera: FlyCamera,
    music: Option<MusicSource>,
    cursor_grabbed: bool,
    last_frame: std::time::Instant,
    gpu: Option<GpuState>,
}

impl App {
    fn new() -> Self {
        let tokio_runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        let music = match tokio_runtime.block_on(MusicSource::load()) {
            Ok(music) => Some(music),
            Err(err) => {
                eprintln!("running silent: {err}");
                None
            }
        };
        Self {
            tokio_runtime,
            camera: FlyCamera::new(Vec3::new(0.0, 1.5, 5.0)),
            music,
            cursor_grabbed: true,
            last_frame: std::time::Instant::now(),
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

        let depth = device.create_depth_texture(width, height);
        let shader = device.create_shader("music_sphere", SOFT_BODY_SHADER);
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

        // The sphere never deforms — build its buffers once.
        let mesh = icosphere(2);
        let positions: Vec<Vec3> = mesh
            .vertices
            .iter()
            .map(|v| SPHERE_CENTER + *v * SPHERE_RADIUS)
            .collect();
        let (sphere_vertex_bytes, sphere_index_bytes, sphere_index_count) =
            soft_body_render_buffers(&positions, &mesh.faces);
        let sphere_vertex_buffer =
            device.create_buffer(sphere_vertex_bytes.len(), BufferUsage::Vertex);
        device.write_buffer(&sphere_vertex_buffer, &sphere_vertex_bytes);
        let sphere_index_buffer =
            device.create_buffer(sphere_index_bytes.len(), BufferUsage::Index);
        device.write_buffer(&sphere_index_buffer, &sphere_index_bytes);

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
            sphere_vertex_buffer,
            sphere_index_buffer,
            sphere_index_count,
        });
    }

    fn on_redraw(&mut self, window: &Window, input: &InputState) {
        let Some(gpu) = &self.gpu else {
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

        let aspect = window.width() as f32 / window.height().max(1) as f32;
        let camera = self.camera.camera(aspect);

        // The camera pose *is* the listener pose — same `Motor3`, same
        // local-forward-`+X` convention (see the module doc).
        if let Some(music) = &mut self.music {
            music.refill(&Listener {
                frame: camera.frame,
            });
        }

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
            pass.set_vertex_buffer(0, &gpu.sphere_vertex_buffer);
            pass.set_index_buffer_u16(&gpu.sphere_index_buffer);
            pass.draw_indexed(0..gpu.sphere_index_count);
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
    run_windowed_app("Meridian Engine — Music Sphere", 1024, 768, App::new())
        .expect("windowed app exited with an error");
}
