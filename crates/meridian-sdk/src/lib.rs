//! Single application-facing entry point for Meridian-Engine.
//!
//! Layer: interface; bounded context: runtime (`craftbook.toml`,
//! docs/context-map.md).
//!
//! **Applications depend on this crate alone.** Everything an app needs
//! — spatial math, physics bodies, graphics scene/material types, the
//! windowed-app loop, audio types, asset decoding, and `engine-core`'s
//! own [`Runtime`] — is re-exported from here; nothing in `examples/`
//! reaches into `meridian-gac-core`/`meridian-physics-core`/
//! `meridian-graphics-core`/etc. directly. This is deliberately
//! different from `engine-core`'s own boundary (rule 7:
//! `meridian-engine-core` is the only crate allowed to depend on every
//! `*-core`, because it's where cross-`*-core` *coordination logic*
//! lives): this crate depends on the same set of crates for a different
//! reason — application ergonomics and driver access `engine-core`
//! itself is forbidden from having.
//!
//! Design notes: docs/notes/meridian-sdk.md (`lib`).

#![forbid(unsafe_code)]

mod any_image_decoder;
pub mod assets;
pub mod camera;
pub mod dsl;
pub mod render;
pub mod scene;

// -- meridian-asset-core: decoding --
pub use any_image_decoder::AnyImageDecoder;
pub use assets::{AssetCache, AudioTrack, load_audio_track, load_image_asset};
pub use camera::{FlyCamera, look_at_rotor};
pub use dsl::dsl_core;
pub use meridian_asset_core::{Decoder, ImageAsset};
pub use meridian_asset_png::PngDecoder;
pub use meridian_asset_symphonia::{
    AnyAudioDecoder, AudioAsset, DecodeStrategy, StreamingAudioDecoder, open_audio,
};
// -- meridian-audio-app: the output port --
pub use meridian_audio_app::AudioSink;
// -- meridian-audio-core: spatial audio --
pub use meridian_audio_core::{
    AcousticMedium, AttenuationModel, BinauralRenderer, Channel, Declicker, DspNode, Emitter,
    Listener, Mixer, SpeakerLayout,
};
// -- meridian-audio-driver: the platform audio adapter (composition root) --
pub use meridian_audio_driver::{AudioStream, open_output};
// -- meridian-compute-driver: the platform compute adapter (composition root) --
pub use meridian_compute_driver::{CpuComputeDevice, GpuComputeDevice, cpu_context, gpu_context};
// -- meridian-compute-runtime: dispatch ports --
pub use meridian_compute_runtime::{ComputeContext, ComputeDevice, GpuBackend};
// -- meridian-engine-core: Runtime — the single frame-work entry point
// (see this crate's own module doc's "Runtime is the single frame-work
// entry point" section) --
pub use meridian_engine_core::{
    AudioSubsystem, ComputeStage, EventSystem, FrameScheduler, PhysicsDispatchMode,
    PhysicsStepStage, PhysicsSubsystem, Runtime, RuntimeState, Stage, StageContext, StageId,
};
// -- meridian-foundation: logging, crash reporting --
pub use meridian_foundation::{
    CrashReportConfig, EngineError, crash_reporting, log_error, log_info, log_warn, logging,
};
// -- meridian-gac-core: spatial math --
pub use meridian_gac_core::{Aabb, Bivector3, Motor3, Projection, Rotor, Vec3};
// -- meridian-graphics-core: scene/material/lighting/submission --
pub use meridian_graphics_core::{
    Camera, Frustum, Light, Material, MaterialHandle, MaterialRegistry, MeshHandle, MeshRegistry,
    MeshSource, Renderable3d, Scene2d, Scene3d, TextureHandle,
};
// -- meridian-graphics-driver: the GPU device a windowed app opens once --
pub use meridian_graphics_driver::{
    BloomPass, CommandBuffer, DEFAULT_SHADOW_MAP_SIZE, DepthTexture, Device, DeviceError,
    DrawBuffers, GpuFrameRecorder, GpuTextureStore, RenderPass, SceneRenderer, Surface,
    TextureRegistry, submit_scene3d,
};
// -- meridian-physics-core: rigid bodies --
pub use meridian_physics_core::{
    BroadPhase, ColliderShape, ConstraintSolver, Contact, Integrator, NarrowPhase, RigidBody,
};
// -- meridian-platform-core: windowing, input, time --
pub use meridian_platform_core::{
    AppHandler, Clock, InputState, KeyCode, MouseButton, Time, Window, run_windowed_app,
};
// -- meridian-rendering-app: the GPU ports --
pub use meridian_rendering_app::{
    FrameOutcome, FrameRecorder, FrameRequest, MeshStore, TextureStore,
};
/// The retained UI runtime (ADR 018); generated `.mel` glue names it
/// through here.
pub use meridian_ui_core as ui_core;
pub use render::{RenderStage, render_frame};
pub use scene::{
    GraphicsSession, cube_mesh_source, ground_mesh_source, icosphere_mesh_source,
    pyramid_mesh_source,
};
