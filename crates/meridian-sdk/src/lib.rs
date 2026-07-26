//! Application-facing composition layer over `engine-core`'s `Runtime`.
//!
//! **Not a hub for domain logic.** `physics-core`/`audio-core`/etc's
//! actual algorithms stay in `engine-core` (docs/dependency-rules.md
//! rule 7 — the one crate allowed to depend on every `*-core`). This
//! crate's own job is narrower and different: composing
//! `engine-core::Runtime` with the `*-driver` crates `engine-core` is
//! forbidden from touching (`graphics-driver`, in this crate's current
//! scope) into a real windowed application loop, plus [`pipeline`]'s
//! composable [`Stage`] system for sequencing per-frame work — physics,
//! audio, rendering, or anything an application defines itself — without
//! `engine-core` dictating a single fixed pipeline shape (see
//! [`pipeline`]'s own module doc for why that distinction matters: it's
//! the direct fix for `engine-core::AudioSubsystem::mix`'s "one
//! opinionated path, not the pipeline every consumer must fit"
//! limitation).
//!
//! `meridian-sdk` depends on `meridian-engine-core` for the actual
//! physics/audio *logic* (via [`pipeline::PhysicsStepStage`], a thin
//! wrapper around `engine_core::PhysicsSubsystem::step` — not a
//! reimplementation; see CLAUDE.md's "don't drag another crate's logic
//! into your own" rule) and on `meridian-graphics-driver`/
//! `meridian-graphics-core` for its own render-stage building blocks —
//! edges `engine-core` itself can never take. It does **not** depend on
//! `physics-core`/`audio-core`/`ecs-core` directly: everything this
//! crate's own code needs from them, it reaches through `engine-core`'s
//! already-public types.

pub mod pipeline;

pub use pipeline::{Pipeline, PipelineState, PhysicsStepStage, Stage, StageContext, StageId};
