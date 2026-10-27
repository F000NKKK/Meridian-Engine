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

pub mod assets;
pub mod camera;
pub mod dsl;
pub mod render;
pub mod scene;

pub use assets::{AssetCache, AudioTrack, load_audio_track, load_image_asset};
pub use camera::{FlyCamera, look_at_rotor};
pub use dsl::dsl_core;
pub use render::{RenderStage, render_frame};
pub use scene::{
    GraphicsSession, cube_mesh_source, ground_mesh_source, icosphere_mesh_source,
    pyramid_mesh_source,
};

// -- meridian-gac-core: spatial math --
pub use meridian_gac_core::{Aabb, Bivector3, Motor3, Projection, Rotor, Vec3};

// -- meridian-physics-core: rigid bodies --
pub use meridian_physics_core::{
    BroadPhase, ColliderShape, ConstraintSolver, Contact, Integrator, NarrowPhase, RigidBody,
};

// -- meridian-graphics-core: scene/material/lighting/submission --
pub use meridian_graphics_core::{
    BloomPass, Camera, DEFAULT_SHADOW_MAP_SIZE, DrawBuffers, Frustum, Light, Material,
    MaterialHandle, MaterialRegistry, MeshHandle, MeshRegistry, MeshSource, Renderable3d, Scene2d,
    Scene3d, SceneRenderer, TextureHandle, TextureRegistry, submit_scene3d,
};

// -- meridian-graphics-driver: the GPU device a windowed app opens once --
pub use meridian_graphics_driver::{
    CommandBuffer, DepthTexture, Device, DeviceError, RenderPass, Surface,
};

// -- meridian-platform-core: windowing, input, time --
pub use meridian_platform_core::{
    AppHandler, Clock, InputState, KeyCode, MouseButton, Time, Window, run_windowed_app,
};

// -- meridian-audio-core: spatial audio --
pub use meridian_audio_core::{
    AcousticMedium, AttenuationModel, AudioOutput, BinauralRenderer, Channel, Declicker, DspNode,
    Emitter, Listener, Mixer, SpeakerLayout,
};

// -- meridian-asset-core: decoding --
pub use meridian_asset_core::{
    AnyAudioDecoder, AnyImageDecoder, AudioAsset, DecodeStrategy, Decoder, ImageAsset,
    StreamingAudioDecoder, open_audio,
};

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
