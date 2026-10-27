//! Single application-facing entry point for Meridian-Engine.
//!
//! Layer: interface; bounded context: runtime (`craftbook.toml`,
//! docs/context-map.md).
//!
//! **Applications depend on this crate alone.** Everything an app needs
//! — spatial math, physics bodies, graphics scene/material types, the
//! windowed-app loop, audio types, asset decoding, and `runtime-app`'s
//! own [`Runtime`] — is re-exported from here; nothing in `examples/`
//! reaches into `meridian-gac`/`meridian-physics-domain`/
//! `meridian-rendering-domain`/etc. directly. This is deliberately
//! different from `runtime-app`'s own boundary (rule 7:
//! `meridian-runtime-app` is the only crate allowed to depend on every
//! `*-core`, because it's where cross-`*-core` *coordination logic*
//! lives): this crate depends on the same set of crates for a different
//! reason — application ergonomics and driver access `runtime-app`
//! itself is forbidden from having.
//!
//! Design notes: docs/notes/meridian-sdk.md (`lib`).

#![forbid(unsafe_code)]

mod any_image_decoder;
pub mod assets;
pub mod dsl;
mod fly_controls;
mod overlay_builder;
pub mod render;
mod runtime;
pub mod scene;

pub use any_image_decoder::AnyImageDecoder;
pub use assets::{AssetCache, AudioTrack, load_audio_track, load_image_asset};
pub use dsl::dsl_core;
pub use fly_controls::fly_controls;
pub use meridian_asset_domain::{Decoder, ImageAsset};
pub use meridian_asset_png::PngDecoder;
pub use meridian_asset_symphonia::{
    AnyAudioDecoder, AudioAsset, DecodeStrategy, StreamingAudioDecoder, open_audio,
};
// -- meridian-audio-app: the output port and audio state --
pub use meridian_audio_app::{AudioSink, AudioSubsystem};
// -- meridian-audio-cpal: the platform audio adapter --
pub use meridian_audio_cpal::{AudioStream, open_output};
// -- meridian-audio-domain: spatial audio --
pub use meridian_audio_domain::{
    AcousticMedium, AttenuationModel, BinauralRenderer, Channel, Declicker, DspNode, Emitter,
    Listener, Mixer, SpeakerLayout,
};
// -- meridian-compute-app: dispatch ports --
pub use meridian_compute_app::{ComputeContext, ComputeDevice, GpuBackend};
// -- meridian-compute-wgpu: the platform compute adapter --
pub use meridian_compute_wgpu::{CpuComputeDevice, GpuComputeDevice, cpu_context, gpu_context};
// -- meridian-diagnostics: errors and the log kernel --
pub use meridian_diagnostics::{EngineError, log_error, log_info, log_warn, logging};
// -- meridian-diagnostics-fs: the log file and the crash report --
pub use meridian_diagnostics_fs::{CrashReportConfig, FileLog, FileLogConfig, install_crash_hook};
// -- meridian-gac: spatial math --
pub use meridian_gac::{Aabb, Bivector3, Motor3, Projection, Rotor, Vec3};
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
// -- meridian-rendering-app: the GPU ports --
pub use meridian_rendering_app::{
    FrameOutcome, FrameRecorder, FrameRequest, MeshStore, RenderSession, TextureStore,
};
// -- meridian-rendering-domain: scene/material/lighting/submission --
pub use meridian_rendering_domain::{
    Camera, Camera2d, ClipRect, ClipSpan, FlyCamera, FlyControls, Frustum, Light, Material,
    MaterialHandle, MaterialRegistry, MeshHandle, MeshRegistry, MeshSource, Overlay2d,
    Renderable3d, Scene2d, Scene3d, Sprite, TextureHandle, cube_mesh_source, ground_mesh_source,
    icosphere_mesh_source, look_at_rotor, pyramid_mesh_source, smooth_normals,
};
// -- meridian-rendering-wgpu: the GPU device a windowed app opens once --
pub use meridian_rendering_wgpu::{
    BloomPass, CommandBuffer, DEFAULT_SHADOW_MAP_SIZE, DepthTexture, Device, DeviceError,
    DrawBuffers, GpuFrameRecorder, GpuTextureStore, RenderPass, SceneRenderer, Surface,
    TextureRegistry, submit_scene3d,
};
// -- meridian-runtime-app: Runtime, the single frame-work entry point --
pub use meridian_runtime_app::{
    ComputeStage, EventSystem, Job, JobExecutor, PhysicsStepStage, Runtime, Shared, Stage, StageId,
};
// -- meridian-runtime-threads: the threaded job executor --
pub use meridian_runtime_threads::TaskExecutor;
/// The retained UI runtime (ADR 018), named by generated `.mel` glue.
pub use meridian_ui_domain as ui_core;
pub use render::{RenderStage, compose_frame, render_frame};
pub use runtime::parallel_runtime;
pub use scene::GraphicsSession;
