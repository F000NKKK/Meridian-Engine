//! Three glowing shapes — a sphere, a cube and a pyramid — orbiting a
//! common center above a textured floor, each spinning on its own axis,
//! each playing its own music track from a *different* container/codec
//! format (`demo-music.mp3`/`.opus`/`.ogg` — real files under
//! `examples/assets/audio/`, identified by signature via
//! `asset-core::AnyAudioDecoder`, never by extension — see ADR 013),
//! and each textured from a different real image file/format
//! (`examples/assets/textures/*.{png,bmp}` — signature-sniffed the same
//! way via `asset-core::AnyImageDecoder`).
//!
//! Rendering goes entirely through `graphics-core`'s submission bridge
//! (`examples::GraphicsBase` bundles `SceneRenderer`/`BloomPass`/the
//! three registries — the same base `physic_figures` builds): the floor
//! is a real Blinn-Phong-lit material under one directional light +
//! ambient; each shape is `unlit` + `emissive` in its own color, so it
//! always reads as glowing regardless of scene lighting, and
//! `BloomPass` turns that emissive value into a real halo — see
//! `meridian_graphics_core::bloom`'s module doc for the shader details
//! (separable Gaussian blur, additive composite).
//!
//! A free-fly camera (WASD + mouse, cursor grabbed on launch, Escape
//! toggles the grab) doubles as the audio listener — same `Motor3`
//! frame, same local-forward-`+X` convention `audio-core::Listener`
//! uses — so all three tracks pan/attenuate through
//! `audio-core::BinauralRenderer` as you fly around, mixed down to one
//! output stream in a single `render` call per frame (all three source
//! files happen to share a sample rate, so one `BinauralRenderer`/
//! `AudioOutput` pair suffices — see [`load_music_tracks`]).
//!
//! Run with:
//!   ./build.sh run magic_figures

use std::collections::VecDeque;

use meridian_asset_core::{AudioAsset, DecodeStrategy, StreamingAudioDecoder, open_audio};
use meridian_audio_core::{
    AcousticMedium, AudioOutput, BinauralRenderer, Declicker, DspNode, Emitter, Listener,
    SpeakerLayout,
};
use meridian_examples::{
    FlyCamera, GraphicsBase, cube_mesh_source, ground_mesh_source, icosphere_mesh_source,
    look_at_rotor, pyramid_mesh_source,
};
use meridian_gac_core::{Motor3, Rotor, Vec3};
use meridian_graphics_core::{DrawBuffers, Light, Material, Renderable3D, Scene3D, submit_scene3d};
use meridian_graphics_driver::Device;
use meridian_platform_core::{AppHandler, InputState, KeyCode, Window, run_windowed_app};

const ORBIT_RADIUS: f32 = 3.2;
const ORBIT_HEIGHT: f32 = 2.0;
/// Full orbit period, seconds — slow enough to watch each shape's own
/// spin independently of the group's motion.
const ORBIT_PERIOD: f32 = 24.0;

/// 10 ms of audio per mixed block — the listener pose is re-sampled at
/// 100 Hz, well past the point where parameter updates read as steps.
const CHUNK_SECONDS: f32 = 0.01;
/// ~80 ms ring: generous headroom so a dropped render frame never
/// drains it (see `audio-driver`'s anti-click fade for the fallback if
/// it ever does) — see the former `music_sphere` example's identical
/// note, now superseded by this one.
const RING_SECONDS: f32 = 0.08;

/// One shape's fixed identity: mesh/texture/glow color, its audio file,
/// and its orbital phase offset (120 degrees apart around the shared
/// center).
struct ShapeSpec {
    name: &'static str,
    audio_file: &'static str,
    texture_file: &'static str,
    glow_color: [f32; 3],
    phase: f32,
    /// Radians per second around its own local axis — visually
    /// distinguishes "orbiting" from "spinning in place".
    spin_speed: f32,
}

const SHAPES: [ShapeSpec; 3] = [
    ShapeSpec {
        name: "cube",
        audio_file: "assets/audio/demo-music.mp3",
        texture_file: "assets/textures/cube.bmp",
        glow_color: [0.25, 0.55, 1.0],
        phase: 0.0,
        spin_speed: 0.9,
    },
    ShapeSpec {
        name: "sphere",
        audio_file: "assets/audio/demo-music.opus",
        texture_file: "assets/textures/sphere.png",
        glow_color: [1.0, 0.55, 0.2],
        phase: std::f32::consts::TAU / 3.0,
        spin_speed: 1.4,
    },
    ShapeSpec {
        name: "pyramid",
        audio_file: "assets/audio/demo-music.ogg",
        texture_file: "assets/textures/pyramid.bmp",
        glow_color: [0.35, 0.95, 0.45],
        phase: 2.0 * std::f32::consts::TAU / 3.0,
        spin_speed: -0.7,
    },
];

/// Where one shape's looping mono samples come from — the two arms of
/// `asset-core::AudioAsset`, behind one [`next_mono_chunk`](Self::next_mono_chunk)
/// face. See the former `music_sphere` example for the identical design
/// this is lifted from (now shared across three simultaneous tracks
/// instead of one).
enum Track {
    Memory {
        mono: Vec<f32>,
        cursor: usize,
    },
    Streamed {
        decoder: StreamingAudioDecoder,
        channels: usize,
        queue: VecDeque<f32>,
    },
}

impl Track {
    fn next_mono_chunk(&mut self, frames: usize) -> Vec<f32> {
        let mut chunk = Vec::with_capacity(frames);
        match self {
            Track::Memory { mono, cursor } => {
                for _ in 0..frames {
                    chunk.push(mono[*cursor]);
                    *cursor = (*cursor + 1) % mono.len();
                }
            }
            Track::Streamed {
                decoder,
                channels,
                queue,
            } => {
                while chunk.len() < frames {
                    if let Some(sample) = queue.pop_front() {
                        chunk.push(sample);
                        continue;
                    }
                    match decoder.next_block() {
                        Ok(Some(block)) => {
                            for frame in block.chunks_exact(*channels) {
                                let sum: f32 = frame.iter().map(|&s| s as f32 / 32768.0).sum();
                                queue.push_back(sum / *channels as f32);
                            }
                        }
                        Ok(None) => {
                            if decoder.rewind().is_err() {
                                chunk.resize(frames, 0.0);
                                break;
                            }
                        }
                        Err(err) => {
                            meridian_foundation::log_warn!("stream decode error: {err}");
                            chunk.resize(frames, 0.0);
                            break;
                        }
                    }
                }
            }
        }
        chunk
    }
}

/// Loads one file at `relative_path` (relative to `CARGO_MANIFEST_DIR`)
/// through `asset-core::open_audio`'s strategy-driven front door —
/// `Auto` decodes short tracks eagerly and streams long ones; this
/// handles both arms. Returns the track plus its sample rate (all three
/// callers happen to agree, but nothing here assumes it).
fn load_track(relative_path: &str) -> Result<(Track, u32), String> {
    let full_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), relative_path);
    let bytes = std::fs::read(&full_path).map_err(|e| format!("{full_path}: {e}"))?;
    let asset =
        open_audio(&bytes, &DecodeStrategy::default()).map_err(|e| format!("{full_path}: {e}"))?;
    let (sample_rate, channels) = (asset.sample_rate(), asset.channels().max(1) as usize);
    let track = match asset {
        AudioAsset::Decoded(audio) => {
            let mut mono: Vec<f32> = audio
                .samples
                .chunks_exact(channels)
                .map(|frame| {
                    frame.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32
                })
                .collect();
            // The loop seam (last sample -> first) is an arbitrary
            // discontinuity; fade both edges over ~10 ms so it passes
            // through silence.
            let fade = (sample_rate as usize / 100).min(mono.len() / 2);
            for i in 0..fade {
                let ramp = i as f32 / fade as f32;
                mono[i] *= ramp;
                let end = mono.len() - 1 - i;
                mono[end] *= ramp;
            }
            Track::Memory { mono, cursor: 0 }
        }
        AudioAsset::Streaming(decoder) => Track::Streamed {
            decoder,
            channels,
            queue: VecDeque::new(),
        },
    };
    Ok((track, sample_rate))
}

/// Loads all three [`SHAPES`] tracks. A shape whose file fails to load
/// or decode plays silence (an all-zero `Track::Memory`) rather than
/// aborting the whole example — one bad asset shouldn't take the other
/// two down with it.
fn load_music_tracks() -> (Vec<Track>, u32) {
    let mut sample_rate = 48_000;
    let tracks = SHAPES
        .iter()
        .map(|shape| match load_track(shape.audio_file) {
            Ok((track, rate)) => {
                sample_rate = rate;
                println!("{}: playing {} ({rate} Hz)", shape.name, shape.audio_file);
                track
            }
            Err(err) => {
                meridian_foundation::log_warn!("{}: running silent ({err})", shape.name);
                Track::Memory {
                    mono: vec![0.0; 48_000],
                    cursor: 0,
                }
            }
        })
        .collect();
    (tracks, sample_rate)
}

/// All three tracks, spatialized in one shared `BinauralRenderer` and
/// pushed into one `AudioOutput` — see the module doc for why one
/// stream suffices.
struct MusicRig {
    output: AudioOutput,
    renderer: BinauralRenderer,
    declicker: Declicker,
    tracks: Vec<Track>,
    chunk_frames: usize,
}

impl MusicRig {
    async fn load() -> Result<Self, String> {
        let (tracks, sample_rate) = load_music_tracks();
        let renderer =
            BinauralRenderer::new(sample_rate).with_medium(AcousticMedium::air_sea_level());
        let ring_frames = (sample_rate as f32 * RING_SECONDS) as u32;
        let output = AudioOutput::open(
            &SpeakerLayout::stereo_headphones(),
            sample_rate,
            Some(ring_frames),
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(Self {
            output,
            renderer,
            declicker: Declicker::new(2),
            tracks,
            chunk_frames: (sample_rate as f32 * CHUNK_SECONDS) as usize,
        })
    }

    /// Tops the output ring buffer up with blocks mixed against
    /// `listener`'s current pose and each shape's current
    /// `positions[i]`. Never blocks: pushes only while the stream
    /// reports room for a whole block.
    fn refill(&mut self, listener: &Listener, positions: &[Vec3; 3]) {
        while self.output.can_push(self.chunk_frames * 2) {
            let chunks: Vec<Vec<f32>> = self
                .tracks
                .iter_mut()
                .map(|track| track.next_mono_chunk(self.chunk_frames))
                .collect();
            let sources: Vec<(Emitter, &[f32])> = positions
                .iter()
                .zip(&chunks)
                .map(|(&position, chunk)| {
                    (
                        Emitter {
                            frame: Motor3::translation(position),
                        },
                        chunk.as_slice(),
                    )
                })
                .collect();
            let mut interleaved = self.renderer.render(listener, &sources, self.chunk_frames);
            self.declicker.process(&mut interleaved);
            self.output.push_interleaved(&interleaved);
        }
    }
}

/// This frame's world position for shape `i` — a circular orbit around
/// the origin at `ORBIT_HEIGHT`, `120°` phase-separated.
fn orbit_position(shape: &ShapeSpec, elapsed: f32) -> Vec3 {
    let angle = elapsed / ORBIT_PERIOD * std::f32::consts::TAU + shape.phase;
    Vec3::new(
        ORBIT_RADIUS * angle.cos(),
        ORBIT_HEIGHT,
        ORBIT_RADIUS * angle.sin(),
    )
}

struct App {
    tokio_runtime: tokio::runtime::Runtime,
    camera: FlyCamera,
    music: Option<MusicRig>,
    cursor_grabbed: bool,
    start: std::time::Instant,
    last_frame: std::time::Instant,
    gpu: Option<GpuState>,
}

struct GpuState {
    base: GraphicsBase,
    scene: Scene3D,
    /// Per-shape (mesh index into `scene.renderables`, spin speed) so
    /// `on_redraw` can update each shape's frame every tick without
    /// re-walking `SHAPES`.
    shape_renderable_indices: [usize; 3],
    /// Index into `scene.lights` of each shape's own `Light::Point`, so
    /// `on_redraw` can move it along with the shape every frame.
    point_light_indices: [usize; 3],
}

impl App {
    fn new() -> Self {
        let tokio_runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        let music = match tokio_runtime.block_on(MusicRig::load()) {
            Ok(music) => Some(music),
            Err(err) => {
                meridian_foundation::log_warn!("running silent: {err}");
                None
            }
        };
        Self {
            tokio_runtime,
            camera: FlyCamera::new(Vec3::new(0.0, 2.0, 7.0)),
            music,
            cursor_grabbed: true,
            start: std::time::Instant::now(),
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
        let mut base = GraphicsBase::new(device, surface, width, height);

        let floor_texture = base.load_texture("assets/textures/floor.png");
        let floor_material = base.materials.register(Material {
            albedo: Some(floor_texture),
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        });
        let floor_mesh = base
            .meshes
            .register(ground_mesh_source(14.0, 10.0))
            .expect("floor mesh must be valid");

        let mut renderables = vec![Renderable3D {
            mesh: floor_mesh,
            material: floor_material,
            frame: Motor3::identity(),
            billboard: false,
        }];

        let mut shape_renderable_indices = [0usize; 3];
        for (i, shape) in SHAPES.iter().enumerate() {
            let mesh_source = match shape.name {
                "cube" => cube_mesh_source(0.8),
                "sphere" => icosphere_mesh_source(2, 0.8),
                _ => pyramid_mesh_source(0.85, 1.4),
            };
            let mesh = base
                .meshes
                .register(mesh_source)
                .unwrap_or_else(|e| panic!("{} mesh must be valid: {e}", shape.name));
            let texture = base.load_texture(shape.texture_file);
            let material = base.materials.register(Material {
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
            shape_renderable_indices[i] = renderables.len();
            renderables.push(Renderable3D {
                mesh,
                material,
                frame: Motor3::translation(orbit_position(shape, 0.0)),
                billboard: false,
            });
        }

        // One `Light::Point` per shape, in its own glow color, colocated
        // with the shape — this is what actually casts colored light
        // onto the floor and the other shapes as they orbit; the
        // material's `emissive` alone (feeding `BloomPass`) only makes
        // the shape itself glow, it doesn't illuminate anything nearby.
        // `MAX_LIGHTS = 4` (see `submission.rs`) is exactly 1 directional
        // + 3 shapes, no headroom to spare.
        let mut lights = vec![Light::Directional {
            direction: Motor3::from_rotation_translation(
                look_at_rotor(Vec3::ZERO, Vec3::new(-0.4, -1.0, -0.3)),
                Vec3::ZERO,
            ),
            color: [1.0, 0.96, 0.9],
            intensity: 0.7,
        }];
        let mut point_light_indices = [0usize; 3];
        for (i, shape) in SHAPES.iter().enumerate() {
            point_light_indices[i] = lights.len();
            lights.push(Light::Point {
                position: Motor3::translation(orbit_position(shape, 0.0)),
                color: shape.glow_color,
                intensity: 2.5,
                range: ORBIT_RADIUS * 2.2,
            });
        }

        let scene = Scene3D {
            renderables,
            lights,
            ambient: [0.05, 0.05, 0.06],
            ..Scene3D::default()
        };

        self.gpu = Some(GpuState {
            base,
            scene,
            shape_renderable_indices,
            point_light_indices,
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

        let elapsed = now.duration_since(self.start).as_secs_f32();
        let mut positions = [Vec3::ZERO; 3];
        for (i, shape) in SHAPES.iter().enumerate() {
            let position = orbit_position(shape, elapsed);
            positions[i] = position;
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
                [0.03, 0.03, 0.05, 1.0],
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
        "magic_figures",
    ));
    meridian_foundation::logging::file::init(
        meridian_foundation::logging::file::FileLogConfig::new("magic_figures"),
    );
    run_windowed_app("Meridian Engine — Magic Figures", 1024, 768, App::new())
        .expect("windowed app exited with an error");
}
