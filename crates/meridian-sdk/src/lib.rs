//! Single application-facing entry point for Meridian-Engine.
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
//! **`Runtime` (in `engine-core`) is the single frame-work entry point —
//! this crate no longer has its own separate `pipeline` module.** That
//! used to exist here specifically because the old `engine-core::Runtime`
//! was a fixed sequential physics-then-audio call with no way to extend
//! it (see `engine-core`'s own module doc, "One mechanism, not two," for
//! the full history) — `Runtime` now *is* the `JobGraph`-based, `Stage`-
//! composable mechanism that module used to be, promoted down into
//! `engine-core` where rule 7 already says cross-`*-core` orchestration
//! belongs. What this crate *does* still add on top: rendering. A
//! `Stage` that presents a frame needs a real windowed `Device`/
//! `Surface` (driver state) `engine-core` is forbidden from touching —
//! this crate is where such a `Stage` gets implemented and registered,
//! through the exact same `Stage` trait every other stage uses, not a
//! second parallel mechanism (see CLAUDE.md's "don't drag another
//! crate's logic into your own" rule — `PhysicsStepStage` itself is
//! `engine-core`'s, just re-exported here, not reimplemented).
//!
//! ## What lives here vs. what's re-exported
//!
//! - [`dsl`] — the extensible scene-composition DSL: parse a small
//!   tag-markup document into typed nodes, where a game registers its
//!   own tags via `#[dsl_tag(name = "...")]` rather than picking from a
//!   fixed schema (see that module's own doc for the full three-layer
//!   design and why this replaced an earlier, rejected fixed-schema
//!   approach).
//! - [`assets`] — resource loading: turning a file path into a real,
//!   cached, GPU-registered handle (textures, OBJ meshes) or a decoded,
//!   loopable audio source ([`AudioTrack`]/[`load_audio_track`]).
//!   Deliberately separate from [`scene`]: this module never touches
//!   scene composition, only decode+cache+register (see its own doc
//!   comment for the full `asset-core`/here/`resource-core` layering).
//! - [`scene`]/[`camera`] — windowed-app scaffolding (procedural mesh
//!   builders, `GraphicsBase`, `FlyCamera`) every windowed application
//!   needs identically; not domain logic, just reusable application
//!   plumbing. `GraphicsBase` owns an [`assets::AssetCache`], but the
//!   loading logic itself lives in [`assets`], not here.
//! - [`render`] — [`render_frame`], the shadow-pass/submit/present
//!   sequence every windowed app repeats each redraw, and [`RenderStage`],
//!   the `engine_core::Stage` impl that lets an application register
//!   frame presentation on the *same* `Runtime`/`JobGraph` as physics —
//!   see that module's own doc for why `engine-core` itself can never
//!   provide this (it has no `graphics-driver` access).
//! - Everything else below is a direct `pub use` from the crate that
//!   actually owns it — this crate adds no new types for spatial math,
//!   physics, graphics or audio; it only makes them reachable through
//!   one door.

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
    GraphicsBase, cube_mesh_source, ground_mesh_source, icosphere_mesh_source, pyramid_mesh_source,
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
    MaterialHandle, MaterialRegistry, MeshHandle, MeshRegistry, MeshSource, Renderable3D, Scene2D,
    Scene3D, SceneRenderer, TextureHandle, TextureRegistry, submit_scene3d,
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

// -- meridian-engine-core: Runtime — the single frame-work entry point
// (see this crate's own module doc's "Runtime is the single frame-work
// entry point" section) --
pub use meridian_engine_core::{
    AudioSubsystem, ComputeStage, EventSystem, FrameScheduler, PhysicsComputeStepStage,
    PhysicsStepStage, PhysicsSubsystem, Runtime, RuntimeState, Stage, StageContext, StageId,
};

// -- meridian-foundation: logging, crash reporting --
pub use meridian_foundation::{
    CrashReportConfig, EngineError, crash_reporting, log_error, log_info, log_warn, logging,
};
