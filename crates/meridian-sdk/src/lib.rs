#![doc = include_str!("../spatial-recipes.md")]
//! Single application-facing entry point for Meridian-Engine.
//! Layer: interface; bounded context: runtime (`craftbook.toml`, docs/context-map.md).
//! **Applications depend on this crate alone.** Everything an app needs
//! (spatial math, physics, graphics, the windowed-app loop, audio, asset
//! decoding, `runtime-app`'s own [`Runtime`]) is re-exported from here;
//! nothing in `examples/` reaches into the composed crates, and this crate
//! may depend on drivers `runtime-app` may not. Notes: docs/notes/meridian-sdk.md.
#![forbid(unsafe_code)]
mod any_image_decoder;
mod asset_loader;
pub mod assets;
pub mod content;
pub mod dsl;
mod editor_renderer;
mod fly_controls;
mod headless;
mod model_files;
mod model_loading;
mod overlay_builder;
pub mod persistence;
pub mod render;
mod runtime;
pub mod scene;

pub use any_image_decoder::AnyImageDecoder;
pub use asset_loader::{asset_loader, default_asset_loader, load_image};
pub use assets::{
    AssetCache, AudioTrack, load_audio_track, load_audio_track_with, load_image_asset,
};
pub use dsl::dsl_core;
// -- meridian-rendering-domain: scene/material/lighting/submission --
pub use editor_renderer::*;
pub use fly_controls::fly_controls;
pub use meridian_asset_app::{
    AssetLoader, AssetPath, ByteSource, LoadError, Pending, Priority, SourceError, Spawn,
};
pub use meridian_asset_domain::{Decoder, ImageAsset};
pub use meridian_asset_fs::FsSource;
pub use meridian_asset_http::{HttpSource, HttpSourceError};
pub use meridian_asset_png::PngDecoder;
pub use meridian_asset_symphonia::{
    AnyAudioDecoder, AudioAsset, DecodeStrategy, StreamingAudioDecoder, open_audio,
};
// -- meridian-audio-app / -cpal: the output port, audio state and platform adapter --
pub use meridian_audio_app::{AudioScene, AudioSink, AudioSubsystem, BusId, VoiceId};
pub use meridian_audio_cpal::{AudioStream, open_output};
// -- meridian-audio-domain: spatial audio --
pub use meridian_audio_domain::{
    AcousticMedium, AttenuationCurve, AttenuationModel, BinauralRenderer, Channel, Declicker,
    DspNode, Emitter, Listener, MemorySource, Mixer, Repeat, ResampledSource, SampleSource,
    SpeakerLayout, Voice,
};
// -- meridian-compute-app / -wgpu: dispatch ports and the platform adapter --
pub use meridian_compute_app::{ComputeContext, ComputeDevice, GpuBackend};
pub use meridian_compute_wgpu::{CpuComputeDevice, GpuComputeDevice, cpu_context, gpu_context};
// -- meridian-diagnostics / -fs: errors, the log kernel, the log file and the crash report --
pub use meridian_diagnostics::{EngineError, log_error, log_info, log_warn, logging};
pub use meridian_diagnostics_fs::{CrashReportConfig, FileLog, FileLogConfig, install_crash_hook};
// -- meridian-gac: spatial math --
pub use meridian_gac::{Aabb, Bivector3, Motor3, Projection, RotationError, Rotor, Scale3, Vec3};
// -- meridian-physics-app / -compute: the world state and its steppers --
pub use meridian_physics_app::{PhysicsStepper, PhysicsSubsystem, SequentialStepper};
// -- meridian-physics-domain: rigid bodies --
pub use meridian_physics_domain::{
    BroadPhase, ColliderShape, ConstraintSolver, Contact, Integrator, NarrowPhase, RigidBody,
};
pub use meridian_physics_wgpu::BatchedStepper;
// -- meridian-platform-winit: windowing, input, time --
pub use meridian_platform_winit::{
    AppHandler, Clock, CpuCapabilities, InputState, KeyCode, MouseButton, Time, Window,
    run_windowed_app,
};
pub use meridian_rendering_app::{
    FrameOutcome, FrameRecorder, FrameRequest, MeshStore, RenderSession, TextureStore,
};
pub use meridian_rendering_domain::{
    Background, BlendMode, Camera, Camera2d, ClipRect, ClipSpan, EnvironmentMap, FlyCamera,
    FlyControls, Frustum, Light, LightKind, LightSource, Material, MaterialHandle,
    MaterialRegistry, MeshHandle, MeshRegistry, MeshRenderer, MeshSource, Overlay2d, Renderable3d,
    Scene2d, Scene3d, Sky, Sprite, TextureHandle, Viewport3d, cube_mesh_source, extract_scene3d,
    ground_mesh_source, icosphere_mesh_source, look_at_rotor, look_at_upright, pyramid_mesh_source,
    smooth_normals,
};
pub use meridian_rendering_wgpu::{
    BloomPass, CommandBuffer, DEFAULT_SHADOW_MAP_SIZE, DepthTexture, Device, DeviceError,
    GpuFrameRecorder, GpuTextureStore, PassTiming, PreparedBatch, RenderPass, SceneRenderer,
    Surface, TextureRegistry, submit_scene3d,
};
pub use meridian_runtime_app::{
    AnimationStage, AudioStage, ComputeStage, EventSystem, FrameDelta, Job, JobExecutor,
    PhysicsStepStage, Runtime, Shared, Stage, StageId, TransformStage, add_world_stages,
};
pub use meridian_runtime_threads::{BackgroundPool, TaskExecutor};
/// The retained UI runtime (ADR 018), named by generated `.meui` glue.
pub use meridian_ui_domain as ui_core;
pub use meridian_ui_scene2d as ui_scene2d;
pub use render::{RenderStage, compose_frame, compose_frame_with_viewports, render_frame};
pub use runtime::parallel_runtime;
pub use scene::GraphicsSession;
