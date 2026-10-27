# `meridian-engine-core` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

**One mechanism, not two.** Earlier revisions of this crate had a
plain sequential `Runtime::tick` (physics-then-audio, hardcoded
order, no extensibility) *and* a separate `meridian-sdk`-level
`Pipeline` built on `task-core::JobGraph` (composable, dependency-
ordered `Stage`s, fine-grained locking) — two competing answers to
"how does a frame's work get run," with the second only invented
because the first couldn't be extended without engine-core knowing
about every possible consumer's specific need (`BinauralRenderer`'s
per-sample synthesis being the concrete case that broke it). That
split is gone: [`Stage`]/[`StageContext`]/[`Runtime`] (the
`JobGraph`-based mechanism, promoted here from where it used to live
in `meridian-sdk`) *is* this crate's `Runtime` now — there is no
second, lower-level "raw pipeline" type an application reaches for
instead; [`Runtime::tick`]/[`Runtime::tick_only`] are the only
per-frame entry points, and each rebuilds and runs a fresh `JobGraph`
every call internally (see
[`Runtime::tick`]'s own doc for why a fresh graph, not a cached one).

**Locking: fine-grained, deadlock-free by construction.**
[`RuntimeState`] splits shared state into independently-lockable
pieces ([`World`], [`PhysicsSubsystem`], [`AudioSubsystem`]) so two
stages that only touch *different* resources genuinely don't contend
on each other's lock. The risk with more than one lock is a classic
lock-order-inversion deadlock: stage A locks physics-then-audio
while an independent stage B locks audio-then-physics, running
concurrently. [`StageContext`] makes that inversion structurally
unrepresentable rather than documenting a convention a stage author
could still get wrong: every accessor takes exactly one lock, and
the only ways to hold more than one at once
([`StageContext::physics_and_audio`]/
[`StageContext::world_physics_and_audio`]) always acquire them in
one fixed order (`world`, then `physics`, then `audio`) internally.
There is no API path that lets calling code choose the order itself.

**Extensibility — this is the actual point of the merge.** Physics
([`PhysicsStepStage`], selecting a plain sequential loop or
`meridian-physics-compute`'s batched-dispatch kernels via its
[`PhysicsDispatchMode`] flag — see that type's own doc for when to
reach for which) and arbitrary GPU/CPU compute
([`ComputeStage`], wrapping any `compute-runtime::ComputeKernel` —
`gac-compute`/`physics-compute`'s own kernels included, or a game's
own) are both just [`Stage`]s registered the same way, dependency-
ordered against each other and anything else an application adds.
Rendering is the one thing that *can't* live as a built-in stage
here — presenting a frame needs a real windowed `Device`/`Surface`
(driver state), and this crate deliberately never depends on
`graphics-driver` (only `graphics-core` — see
docs/dependency-rules.md). That isn't a gap in this mechanism,
though: a render-presenting [`Stage`] is exactly what an application
(via `meridian-sdk`, the one crate allowed to hold both `Runtime` and
`*-driver` types) implements and registers itself, through the exact
same `Stage` trait every other stage uses — not a second, parallel
"render pipeline" mechanism. See `meridian-sdk`'s own module docs for
that implementation once it exists.

**`examples/physic_figures` proves this end-to-end** — physics *and*
rendering both go through one `Runtime`: a registered
[`PhysicsStepStage`] and a `meridian-sdk`-implemented render-
presenting `Stage`, the concrete case the previous paragraph
describes. They run at different multiplicities per real display
frame (a fixed-timestep accumulator calls the physics stage 0-8
times to catch up; rendering must run exactly once) — see
[`Runtime::tick_only`]'s own doc for the selective-tick mechanism
that makes both still go through this one `Runtime` rather than
either bypassing it. `examples/magic_figures` still doesn't use
`Runtime` at all: it has no physics bodies (pure kinematic orbit
motion) and needs `audio-core::BinauralRenderer`'s real per-sample
stereo synthesis, which doesn't reduce to a `StageContext`-shaped
`Stage` cleanly (its inputs — orbiting shapes' positions, the
free-fly camera's pose — are plain application state, not anything
`RuntimeState` owns) — see [`AudioSubsystem::mix`]'s own doc comment
for why forcing it through `AudioSubsystem` was rejected instead.
