# ADR 025: Wave 6 — a runtime without state, a composition root without logic

## Status

Accepted. Carries out [ADR 020](020-craftbook-adoption.md) D2 and D3 and
extends [ADR 024](024-wave-5-ports-and-adapters.md).

## Context

After wave 5 the runtime crate (`engine-core`) still owned the world, physics
and audio state behind `Mutex`es (`RuntimeState`), reached three adapters
(`task-core`, `platform-core`, `physics-compute`), and the sdk held scene,
camera and render logic. ADR 020 D2/D3 say what to do; the details below are
decided here.

## Decisions

**D1 — the runtime owns no state; stages hold `Shared` handles.**
`RuntimeState`, `StageContext` and its ordered-lock accessors are removed.
`Stage::run(&mut self)` takes nothing: the composition root builds the state,
wraps what several stages touch in `Shared<T>` (a poison-tolerant
`Arc<Mutex<T>>`), and constructs each stage with the handles it uses. The
reason is the rule, not a convenience: a hub that owns every piece of state
is a hub whatever its locking discipline, and a stage that names its handles
shows its data flow at the construction site.

**D2 — threads are a port declared by the runtime.** `engine-core` declares
`JobExecutor` (`execute(Vec<Job>)`); `meridian-task-core` implements it
(`TaskExecutor`), so `task-core` now depends on `engine-core` (infra to app,
inward). The worker count is passed in; `engine-core` no longer asks
`platform-core` for it. A fake (`testing::InlineExecutor`) and a contract
suite (`testing::run_executor_contract`) cover the port; `task-core` runs the
contract against the real scheduler.

**D3 — physics stepping is a port declared by the physics context.**
`meridian-physics-app` holds `PhysicsSubsystem`, the `PhysicsStepper` port,
`SequentialStepper` and a contract suite. `physics-compute` implements the
port (`BatchedStepper`) and depends on `physics-app`, its own context's app
crate, as rule 2 of dependency-rules.md requires. The old
`PhysicsDispatchMode` flag is gone: the composition root passes the stepper.
`AudioSubsystem` moved to `audio-app` for the same reason: it is audio state,
not runtime logic.

**D4 — the sdk keeps adapter choice and glue, not logic.** It names the
adapters (`parallel_runtime`, `GraphicsSession` over the wgpu recorder, the
platform window) and re-exports the public surface.

**D5 — what stays in the sdk, and why.** The DSL module (`#[dsl_tag]` tags,
`build_scene`) stays: the `dsl-macros` expansion emits `::meridian_sdk::dsl_core`
paths, so the tags can only move together with a macro rework, which belongs
with the wave 7 renames. `assets.rs` (file reading, the image and audio
decoder wiring, the path cache) stays for now: it is composition work until an
`asset-app` crate with a file-source port exists; the pure geometry in it
(smooth normals) moves to `graphics-core`.
