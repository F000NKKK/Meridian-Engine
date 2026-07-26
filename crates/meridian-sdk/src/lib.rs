//! Single application-facing entry point for Meridian-Engine.
//!
//! **Applications depend on this crate alone.** Everything an app needs
//! — spatial math, physics bodies, graphics scene/material types, the
//! windowed-app loop, audio types, asset decoding, and this crate's own
//! [`pipeline`] — is re-exported from here; nothing in `examples/`
//! reaches into `meridian-gac-core`/`meridian-physics-core`/
//! `meridian-graphics-core`/etc. directly. This is deliberately
//! different from `engine-core`'s own boundary (rule 7:
//! `meridian-engine-core` is the only crate allowed to depend on every
//! `*-core`, because it's where cross-`*-core` *coordination logic*
//! lives): this crate depends on the same set of crates for a different
//! reason — application ergonomics and driver access `engine-core`
//! itself is forbidden from having — and adds no new coordination logic
//! of its own beyond what [`pipeline`] provides (thin `Stage` wrappers
//! around methods `engine-core` already exposes, e.g.
//! [`PhysicsStepStage`] around `engine_core::PhysicsSubsystem::step` —
//! not a reimplementation; see CLAUDE.md's "don't drag another crate's
//! logic into your own" rule). See docs/dependency-rules.md's own note
//! on this edge for the full reasoning.
//!
//! ## What lives here vs. what's re-exported
//!
//! - [`pipeline`] — this crate's own composable, `JobGraph`-based frame
//!   pipeline (see that module's doc for why `engine-core` itself
//!   couldn't be the place for this — it's the direct fix for
//!   `AudioSubsystem::mix`'s "one opinionated path, not the pipeline
//!   every consumer must fit" limitation).
//! - [`scene`]/[`camera`] — windowed-app scaffolding (mesh builders,
//!   `GraphicsBase`, `FlyCamera`) every windowed application needs
//!   identically; not domain logic, just reusable application plumbing.
//! - Everything else below is a direct `pub use` from the crate that
//!   actually owns it — this crate adds no new types for spatial math,
//!   physics, graphics or audio; it only makes them reachable through
//!   one door.

pub mod camera;
pub mod pipeline;
pub mod scene;

pub use camera::{FlyCamera, look_at_rotor};
pub use pipeline::{PhysicsStepStage, Pipeline, PipelineState, Stage, StageContext, StageId};
pub use scene::{
    GraphicsBase, cube_mesh_source, ground_mesh_source, icosphere_mesh_source, load_image_asset,
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
    BloomPass, Camera, DrawBuffers, Frustum, Light, Material, MaterialHandle, MaterialRegistry,
    MeshHandle, MeshRegistry, MeshSource, Renderable3D, Scene2D, Scene3D, SceneRenderer,
    TextureHandle, TextureRegistry, submit_scene3d,
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
    AnyAudioDecoder, AnyImageDecoder, AudioAsset, DecodeStrategy, Decoder, ImageData,
    StreamingAudioDecoder, open_audio,
};

// -- meridian-engine-core: Runtime/SubsystemManager, for apps that want
// the simpler non-pipeline composition (see that crate's own module doc
// for when `Runtime::tick` fits vs. when `pipeline::Pipeline` does) --
pub use meridian_engine_core::{
    AudioSubsystem, EventSystem, FrameCompleted, FrameScheduler, PhysicsSubsystem, Runtime,
    SubsystemManager,
};

// -- meridian-foundation: logging, crash reporting --
pub use meridian_foundation::{
    CrashReportConfig, EngineError, crash_reporting, log_error, log_info, log_warn, logging,
};
