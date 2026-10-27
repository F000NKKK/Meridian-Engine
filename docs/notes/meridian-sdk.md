# `meridian-sdk` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

**`Runtime` (in `engine-core`) is the single frame-work entry point —
this crate no longer has its own separate `pipeline` module.** That
used to exist here specifically because the old `engine-core::Runtime`
was a fixed sequential physics-then-audio call with no way to extend
it (see `engine-core`'s own module doc, "One mechanism, not two," for
the full history) — `Runtime` now *is* the `JobGraph`-based, `Stage`-
composable mechanism that module used to be, promoted down into
`engine-core` where rule 7 already says cross-`*-core` orchestration
belongs. What this crate *does* still add on top: rendering. A
`Stage` that presents a frame needs a real windowed `Device`/
`Surface` (driver state) `engine-core` is forbidden from touching —
this crate is where such a `Stage` gets implemented and registered,
through the exact same `Stage` trait every other stage uses, not a
second parallel mechanism (see CLAUDE.md's "don't drag another
crate's logic into your own" rule — `PhysicsStepStage` itself is
`engine-core`'s, just re-exported here, not reimplemented).

## What lives here vs. what's re-exported

- [`dsl`] — the extensible scene-composition DSL: parse a small
tag-markup document into typed nodes, where a game registers its
own tags via `#[dsl_tag(name = "...")]` rather than picking from a
fixed schema (see that module's own doc for the full three-layer
design and why this replaced an earlier, rejected fixed-schema
approach).
- [`assets`] — resource loading: turning a file path into a real,
cached, GPU-registered handle (textures, OBJ meshes) or a decoded,
loopable audio source ([`AudioTrack`]/[`load_audio_track`]).
Deliberately separate from [`scene`]: this module never touches
scene composition, only decode+cache+register (see its own doc
comment for the full `asset-core`/here/`resource-core` layering).
- [`scene`]/[`camera`] — windowed-app scaffolding (procedural mesh
builders, `GraphicsBase`, `FlyCamera`) every windowed application
needs identically; not domain logic, just reusable application
plumbing. `GraphicsBase` owns an [`assets::AssetCache`], but the
loading logic itself lives in [`assets`], not here.
- [`render`] — [`render_frame`], the shadow-pass/submit/present
sequence every windowed app repeats each redraw, and [`RenderStage`],
the `engine_core::Stage` impl that lets an application register
frame presentation on the *same* `Runtime`/`JobGraph` as physics —
see that module's own doc for why `engine-core` itself can never
provide this (it has no `graphics-driver` access).
- Everything else below is a direct `pub use` from the crate that
actually owns it — this crate adds no new types for spatial math,
physics, graphics or audio; it only makes them reachable through
one door.

## `dsl`

- [`meridian_dsl_core`] (re-exported here as [`dsl_core`]) — the
domain-blind text parser (`<Tag attr="value">...</Tag>` ->
`Element`) plus [`dsl_core::DslTag`]/[`dsl_core::DslRegistry`], the
generic "typed thing buildable from attributes" machinery. Never
changes when a new tag is added.
- [`dsl_tag`] (the `#[dsl_tag(name = "...")]` attribute macro,
re-exported from `meridian-dsl-macros`) — code generation: turn a
plain struct into a [`dsl_core::DslTag`] impl, one required
attribute per field (`Option<T>` fields are optional). This is how
a game adds `<MyGameSpecificWidget hp="100" />` without touching
this crate at all.
- This module's own [`Scene`]/[`Entity`]/[`Mesh`]/[`Material`]/
[`RigidBody`]/[`Transform`] — built-in tags for the composition
primitives every application needs, registered by
[`default_registry`]. An application that doesn't want one of
these can build its own `DslRegistry` from scratch instead of
calling [`default_registry`].

## Scope, deliberately

This DSL describes scene/entity composition only — no window title,
no logging config, no crash-report directory (see the earlier design
discussion that scoped this to Phase 1). An application still writes
that part in plain Rust `main()`, same as today.

Parse errors ([`dsl_core::ParseError`]) and tag-build errors
([`dsl_core::TagParseError`]) both implement
`meridian_foundation::EngineError` (see that trait's own doc) — a
caller that lets one propagate into a panic (rather than, say,
falling back to an empty scene) gets it captured by
`crash_reporting`'s post-mortem for free, same as every other engine
error; nothing about this module is a silent dead end.
