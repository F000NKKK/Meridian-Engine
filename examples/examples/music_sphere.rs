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
//! **Rendering converges on `graphics-core`'s submission bridge**
//! (`SceneRenderer`/`MeshRegistry`/`MaterialRegistry`/`TextureRegistry`)
//! instead of hand-rolling pipelines/buffers the way this example used
//! to: a checkerboard floor (a procedurally generated `ImageData`
//! uploaded through `TextureRegistry` — no image *file* needed for a
//! flat two-color pattern, though `TextureRegistry::upload` takes any
//! decoded asset the same way), a plain colored cube, and the music
//! sphere itself as an unlit, emissive material — `BloomPass` turns
//! that emissive value into a real glow around it. One directional
//! light plus the scene's ambient does the (Blinn-Phong) shading on the
//! floor and cube.
//!
//! Controls as in the soft-body examples: WASD + mouse (cursor grabbed
//! on launch), Space/Ctrl up/down, Shift faster, Escape toggles the
//! cursor grab.
//!
//! Run with:
//!   ./build.sh run music_sphere

use meridian_asset_core::{
    AnyAudioDecoder, AudioAsset, DecodeStrategy, ImageData, StreamingAudioDecoder, open_audio,
};
use meridian_audio_core::{
    AcousticMedium, AudioOutput, BinauralRenderer, Declicker, DspNode, Emitter, Listener,
    SpeakerLayout,
};
use meridian_examples::FlyCamera;
use meridian_gac_core::{Motor3, Vec3, icosphere};
use meridian_graphics_core::{
    BloomPass, DrawBuffers, Light, Material, MaterialRegistry, MeshRegistry, MeshSource,
    Renderable3D, Scene3D, SceneRenderer, TextureRegistry, submit_scene3d,
};
use meridian_graphics_driver::{DepthTexture, Device, Surface};
use meridian_platform_core::{AppHandler, InputState, KeyCode, Window, run_windowed_app};

/// Builds a [`MeshSource`] for an icosphere of the given `radius`,
/// centered at its own local origin (world placement is
/// `Renderable3D::frame`'s job — `Motor3` has no scale component, so
/// radius has to be baked into the mesh itself, not applied per
/// instance). Normals are the unit-length vertex directions, UVs an
/// equirectangular projection.
fn icosphere_mesh_source(subdivisions: u32, radius: f32) -> MeshSource {
    let mesh = icosphere(subdivisions);
    let positions: Vec<[f32; 3]> = mesh
        .vertices
        .iter()
        .map(|v| [v.x * radius, v.y * radius, v.z * radius])
        .collect();
    let normals: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| [v.x, v.y, v.z]).collect();
    let uvs: Vec<[f32; 2]> = mesh
        .vertices
        .iter()
        .map(|v| {
            let n = v.normalize();
            let u = 0.5 + n.z.atan2(n.x) / std::f32::consts::TAU;
            let v = 0.5 - n.y.asin() / std::f32::consts::PI;
            [u, v]
        })
        .collect();
    let mut indices = Vec::new();
    for face in &mesh.faces {
        for (a, b, c) in face.triangles() {
            indices.push(a as u32);
            indices.push(b as u32);
            indices.push(c as u32);
        }
    }
    MeshSource {
        positions,
        normals,
        uvs,
        indices,
    }
}

/// A unit cube (half-extent 1 on every axis), one set of 4 vertices per
/// face so each face gets its own flat normal and a full `[0,1]` UV.
fn cube_mesh_source() -> MeshSource {
    const FACES: [([f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]); 6] = [
        // (normal, corner00, corner10, corner11, corner01) — CCW as seen
        // from outside the cube along `normal`, matching this crate's
        // `FrontFace::Ccw` convention.
        (
            [1.0, 0.0, 0.0],
            [1.0, -1.0, -1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, -1.0],
        ),
        (
            [-1.0, 0.0, 0.0],
            [-1.0, -1.0, 1.0],
            [-1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, 1.0, 1.0],
        ),
        (
            [0.0, 1.0, 0.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ),
        (
            [0.0, -1.0, 0.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, -1.0, -1.0],
            [-1.0, -1.0, -1.0],
        ),
        (
            [0.0, 0.0, 1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ),
        (
            [0.0, 0.0, -1.0],
            [1.0, -1.0, -1.0],
            [-1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
        ),
    ];

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for (normal, c00, c10, c11, c01) in FACES {
        let base = positions.len() as u32;
        for corner in [c00, c10, c11, c01] {
            positions.push(corner);
            normals.push(normal);
        }
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    MeshSource {
        positions,
        normals,
        uvs,
        indices,
    }
}

/// A flat quad in the local `y = 0` plane, `half_size` from center to
/// edge, its UVs tiled `uv_tiles` times across — the floor's mesh, with
/// world placement (and any further scale) left to
/// `Renderable3D::frame`.
fn ground_mesh_source(half_size: f32, uv_tiles: f32) -> MeshSource {
    MeshSource {
        positions: vec![
            [-half_size, 0.0, -half_size],
            [half_size, 0.0, -half_size],
            [half_size, 0.0, half_size],
            [-half_size, 0.0, half_size],
        ],
        normals: vec![[0.0, 1.0, 0.0]; 4],
        uvs: vec![
            [0.0, 0.0],
            [uv_tiles, 0.0],
            [uv_tiles, uv_tiles],
            [0.0, uv_tiles],
        ],
        // Winding front-facing from +Y — see `examples::ground_quad_buffers`'s
        // identical note on why [0,1,2, 0,2,3] would cull as invisible here.
        indices: vec![0, 2, 1, 0, 3, 2],
    }
}

/// A procedural black/white checkerboard, `cells`x`cells` squares across
/// a `resolution`x`resolution` RGBA8 image — no external texture asset
/// needed for a pattern this simple to generate directly (see the
/// module doc; `TextureRegistry::upload` takes any decoded `ImageData`
/// the same way a real loaded file's would arrive).
fn checkerboard_image(resolution: u32, cells: u32) -> ImageData {
    let cell_size = (resolution / cells).max(1);
    let mut pixels = Vec::with_capacity((resolution * resolution * 4) as usize);
    for y in 0..resolution {
        for x in 0..resolution {
            let light = ((x / cell_size) + (y / cell_size)).is_multiple_of(2);
            let shade = if light { 230u8 } else { 30u8 };
            pixels.extend_from_slice(&[shade, shade, shade, 255]);
        }
    }
    ImageData {
        width: resolution,
        height: resolution,
        pixels,
    }
}

const SPHERE_CENTER: Vec3 = Vec3 {
    x: 0.0,
    y: 1.5,
    z: 0.0,
};
const SPHERE_RADIUS: f32 = 0.75;
/// 10 ms of audio per mixed block — the listener pose is re-sampled at
/// 100 Hz, well past the point where parameter updates read as steps.
const CHUNK_SECONDS: f32 = 0.01;
/// Requested ring buffer: ~80 ms. Together with the 10 ms block this
/// bounds pose-to-ear latency at ~90 ms — deliberately generous so a
/// dropped render frame or a grabbed window never drains the ring (an
/// empty ring is a starvation dip; the driver's anti-click fade keeps
/// even that from *clicking*, but headroom keeps it from happening at
/// all). Pose *updates* still land every 10 ms block, so panning stays
/// smooth; only the absolute lag grows, which the ear tolerates far
/// more readily than steps.
const RING_SECONDS: f32 = 0.08;

/// Where the looping track's mono samples come from — the two arms of
/// `asset-core::AudioAsset`, behind one `next_mono_chunk` face.
enum Track {
    /// Small tracks (`Auto` decided eager): the whole downmixed track
    /// in memory, looped by cursor wrap, seam pre-faded.
    Memory { mono: Vec<f32>, cursor: usize },
    /// Large tracks (`Auto` decided streaming — the 91 s demo track's
    /// ~17 MB of PCM qualifies): blocks decoded on demand, rewound at
    /// the end to loop. The loop seam is an unfaded discontinuity here;
    /// the `Declicker` downstream is what absorbs it.
    Streamed {
        decoder: StreamingAudioDecoder,
        channels: usize,
        queue: std::collections::VecDeque<f32>,
    },
}

impl Track {
    /// The next `frames` mono samples, looping seamlessly forever.
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
                            // End of track: loop by rewinding the shared
                            // compressed bytes — no re-read, no copy.
                            if decoder.rewind().is_err() {
                                chunk.resize(frames, 0.0);
                                break;
                            }
                        }
                        Err(err) => {
                            eprintln!("stream decode error: {err}");
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

/// The looping music track, mixed spatially against the current
/// listener pose and topped up into the output stream without blocking.
struct MusicSource {
    output: AudioOutput,
    renderer: BinauralRenderer,
    /// Post-processing safety net: slew-limits the final stereo stream
    /// so no step discontinuity from anywhere in the chain survives as
    /// a click.
    declicker: Declicker,
    track: Track,
    chunk_frames: usize,
}

impl MusicSource {
    /// Opens the first playable file in `examples/assets/audio/` (by
    /// signature, not extension) through `asset-core::open_audio`'s
    /// strategy-driven front door: `Auto` decodes small tracks eagerly
    /// and streams large ones — this example handles both arms.
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
            let asset = match open_audio(&bytes, &DecodeStrategy::default()) {
                Ok(asset) => asset,
                Err(err) => {
                    eprintln!("  {}: {err}", path.display());
                    continue;
                }
            };
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
                    // discontinuity; fade both edges over ~10 ms so it
                    // passes through silence.
                    let fade = (sample_rate as usize / 100).min(mono.len() / 2);
                    for i in 0..fade {
                        let ramp = i as f32 / fade as f32;
                        mono[i] *= ramp;
                        let end = mono.len() - 1 - i;
                        mono[end] *= ramp;
                    }
                    println!(
                        "playing {} ({format:?}, {sample_rate} Hz, {channels} ch, eager)",
                        path.display()
                    );
                    Track::Memory { mono, cursor: 0 }
                }
                AudioAsset::Streaming(decoder) => {
                    println!(
                        "playing {} ({format:?}, {sample_rate} Hz, {channels} ch, streaming)",
                        path.display()
                    );
                    Track::Streamed {
                        decoder,
                        channels,
                        queue: std::collections::VecDeque::new(),
                    }
                }
            };

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
            return Ok(Self {
                output,
                renderer,
                declicker: Declicker::new(2),
                track,
                chunk_frames: (sample_rate as f32 * CHUNK_SECONDS) as usize,
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
            let chunk = self.track.next_mono_chunk(self.chunk_frames);
            let mut interleaved =
                self.renderer
                    .render(listener, &[(emitter, &chunk)], self.chunk_frames);
            self.declicker.process(&mut interleaved);
            self.output.push_interleaved(&interleaved);
        }
    }
}

struct GpuState {
    device: Device,
    surface: Surface,
    depth: DepthTexture,
    renderer: SceneRenderer,
    bloom: BloomPass,
    meshes: MeshRegistry,
    materials: MaterialRegistry,
    textures: TextureRegistry,
    scene: Scene3D,
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
        let renderer = SceneRenderer::new(&device, &surface);
        let bloom = BloomPass::new(&device, width, height, &surface);

        let mut meshes = MeshRegistry::new();
        let mut materials = MaterialRegistry::new();
        let mut textures = TextureRegistry::new();

        let checkerboard = textures.upload(&device, &checkerboard_image(256, 8));
        let floor_material = materials.register(Material {
            albedo: Some(checkerboard),
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        });
        let cube_material = materials.register(Material {
            base_color_factor: [0.25, 0.45, 0.9, 1.0],
            ..Default::default()
        });
        // Unlit + emissive: the sphere always reads as a light source
        // regardless of scene lighting, and its emissive value is
        // exactly what `BloomPass` blooms — see the module doc.
        let sphere_material = materials.register(Material {
            base_color_factor: [1.0, 0.65, 0.25, 1.0],
            unlit: true,
            emissive: [1.0, 0.65, 0.25],
            ..Default::default()
        });

        let floor_mesh = meshes
            .register(ground_mesh_source(15.0, 8.0))
            .expect("floor mesh must be valid");
        let cube_mesh = meshes
            .register(cube_mesh_source())
            .expect("cube mesh must be valid");
        let sphere_mesh = meshes
            .register(icosphere_mesh_source(2, SPHERE_RADIUS))
            .expect("sphere mesh must be valid");

        let scene = Scene3D {
            renderables: vec![
                Renderable3D {
                    mesh: floor_mesh,
                    material: floor_material,
                    frame: Motor3::identity(),
                    billboard: false,
                },
                Renderable3D {
                    mesh: cube_mesh,
                    material: cube_material,
                    frame: Motor3::translation(Vec3::new(-2.5, 1.0, -1.5)),
                    billboard: false,
                },
                Renderable3D {
                    mesh: sphere_mesh,
                    material: sphere_material,
                    frame: Motor3::translation(SPHERE_CENTER),
                    billboard: false,
                },
            ],
            // `Light::Directional::direction` is a `Motor3` whose local
            // +X (see `look_at_rotor`'s convention) *is* the direction
            // light travels; pointing +X at `sun_direction` from the
            // origin gives exactly that rotor, with no translation.
            lights: vec![Light::Directional {
                direction: Motor3::from_rotation_translation(
                    meridian_examples::look_at_rotor(Vec3::ZERO, Vec3::new(-0.4, -1.0, -0.3)),
                    Vec3::ZERO,
                ),
                color: [1.0, 0.96, 0.9],
                intensity: 1.2,
            }],
            ambient: [0.08, 0.08, 0.1],
            ..Scene3D::default()
        };

        self.gpu = Some(GpuState {
            device,
            surface,
            depth,
            renderer,
            bloom,
            meshes,
            materials,
            textures,
            scene,
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

        let aspect = window.width() as f32 / window.height().max(1) as f32;
        let camera = self.camera.camera(aspect);

        // The camera pose *is* the listener pose — same `Motor3`, same
        // local-forward-`+X` convention (see the module doc).
        if let Some(music) = &mut self.music {
            music.refill(&Listener {
                frame: camera.frame,
            });
        }

        gpu.scene.camera = camera;

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
        let draw_buffers: Vec<DrawBuffers>;
        {
            let mut pass =
                commands.begin_render_pass(frame.view(), [0.05, 0.05, 0.08, 1.0], Some(&gpu.depth));
            draw_buffers = submit_scene3d(
                &gpu.device,
                &gpu.renderer,
                &mut pass,
                &gpu.scene,
                &gpu.meshes,
                &gpu.materials,
                &gpu.textures,
            );
        }
        gpu.bloom.apply(
            &gpu.device,
            &mut commands,
            &gpu.renderer,
            &draw_buffers,
            &frame,
        );
        commands.submit();
        frame.present(&gpu.device);

        window.request_redraw();
    }

    fn on_resized(&mut self, width: u32, height: u32) {
        if let Some(gpu) = &mut self.gpu {
            gpu.surface.resize(&gpu.device, width, height);
            gpu.depth = gpu.device.create_depth_texture(width, height);
            gpu.bloom = BloomPass::new(&gpu.device, width, height, &gpu.surface);
        }
    }
}

fn main() {
    meridian_foundation::crash_reporting::install(meridian_foundation::CrashReportConfig::new(
        "music_sphere",
    ));
    meridian_foundation::logging::file::init(
        meridian_foundation::logging::file::FileLogConfig::new("music_sphere"),
    );
    run_windowed_app("Meridian Engine — Music Sphere", 1024, 768, App::new())
        .expect("windowed app exited with an error");
}
