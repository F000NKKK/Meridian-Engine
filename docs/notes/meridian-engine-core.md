# `meridian-engine-core` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

**One mechanism, not two.** [`Stage`]/[`Runtime`] is the only per-frame
entry point: a `Runtime` registers stages with their dependencies and
[`Runtime::tick`]/[`Runtime::tick_only`] run them in dependency order,
independent branches in parallel when the executor has workers. Each call
builds a fresh job list ([`Job`]s are `FnOnce`, consumed once per frame, see
[`Runtime::tick`]).

**No state owner (ADR 025).** Earlier revisions had a `RuntimeState` hub that
owned the world, physics and audio behind `Mutex`es and a `StageContext` that
handed out ordered locks. That hub is gone. The composition root creates the
state and wraps what several stages touch in [`Shared`]; each stage is
constructed with the handles it uses, so every stage names exactly what it
touches and a lock-order inversion needs two stages that each hold two
handles, which the composition root can see in one place. A stage that needs
a second handle holds it as a field; none of the runtime's types lock for it.

**Threads are a port.** The runtime does not name `task-core`: it builds
[`Job`]s (a name, dependency indices into the same list, an action) and hands
them to a [`JobExecutor`]. `meridian-task-core` implements the port
(`TaskExecutor`); [`testing::InlineExecutor`] is the deterministic fake and
[`testing::run_executor_contract`] the suite both pass. The composition root
decides the worker count (`meridian_sdk::parallel_runtime` sizes it from the
detected CPU threads); `engine-core` no longer reaches for `platform-core`.

**Extensibility.** Physics ([`PhysicsStepStage`], which advances a shared
`PhysicsSubsystem` through whichever `PhysicsStepper` it is given: the
sequential loop from `meridian-physics-app` or the batched kernels of
`meridian-physics-compute`) and arbitrary GPU/CPU compute
([`ComputeStage`], wrapping any `compute-runtime::ComputeKernel`) are both
just [`Stage`]s registered the same way. Rendering cannot be a built-in
stage: presenting a frame needs a windowed device (adapter state), so the
composition root (`meridian-sdk::RenderStage`) implements the same `Stage`
trait over the `FrameRecorder` port.

**`examples/physic_figures`** runs physics and rendering through one
`Runtime`: a registered [`PhysicsStepStage`] and the sdk's render stage. They
run at different multiplicities per display frame (a fixed-timestep
accumulator calls the physics stage 0-8 times, rendering exactly once), which
is what [`Runtime::tick_only`] is for. `examples/magic_figures` does not use
`Runtime`: it has no physics bodies and needs `audio-core::BinauralRenderer`'s
per-sample synthesis, whose inputs (orbit positions, the camera pose) are
plain application state; forcing it through `AudioSubsystem::mix` was
rejected.
