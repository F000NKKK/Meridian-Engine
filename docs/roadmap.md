# Roadmap

## Current state

`meridian-foundation`, `meridian-numeric-core`, and `meridian-gac-core` have
real implementations: `Vec3`, `Rotor`, `Motor3`, and the underlying 16-blade
`Multivector` geometric product, with a test suite that cross-checks
rotation against an independent Rodrigues-formula oracle and validates the
parent/child transform-hierarchy milestone below (`cargo test -p
meridian-gac-core`; runnable end to end via `./build.sh run
gac_validation`).

Step 3 (`memory-core`/`task-core`/`platform-core`) is also real and tested:
- `meridian-memory-core` — generational `ResourcePool<T>` (stale-handle
  detection via generation bump on reuse) and `FrameArena<T>`/
  `PersistentArena<T>` (safe bump-style lists, `reset()` retains capacity).
- `meridian-task-core` — `JobGraph`/`Scheduler`: dependency-ordered
  execution across worker threads, panics on a cycle instead of hanging.
  Deliberately a shared mutex-guarded ready-queue rather than per-worker
  lock-free work-stealing deques — see docs/threading-model.md for why
  that's a first step, not the final design.
- `meridian-platform-core` — `Time`/`Clock` and `InputState` (held/
  pressed-this-frame/released-this-frame key state, mouse position+delta)
  are real; `Window` and `DynamicLibrary` are deliberately still stubs —
  see "Not yet decided" below.

Every other crate is still a scaffold: correct name, correct dependency
edges, a one-line doc comment, no implementation. This staged order is
intentional — see "Why implementation is deliberately last" below.

## Why implementation is deliberately last

The riskiest failure mode at this project's current size isn't technical
complexity — it's layers slowly bleeding into each other (a rendering
concept leaking into `ecs-core`, a global manager reappearing in
`asset-core`, a driver crate quietly depending on its own core). Once real
code exists, every such violation costs a refactor. Right now it costs a
one-line correction to [dependency-rules.md](dependency-rules.md). The
priority before writing implementations is keeping that document and the
[ADRs](adr/) accurate and complete.

## Suggested implementation order

1. `meridian-foundation`, `meridian-numeric-core` — scalar/SIMD groundwork
   everything else depends on.
2. `meridian-gac-core` — motors, rotors, transforms. Nothing above this
   layer can be meaningfully tested without it. Build it bottom-up and
   validate before moving on: `Vec3` → `Rotor3` → `Motor3` → a parent/child
   transform-hierarchy composition test, *then* wire `Transform` into
   `ecs-core` (step 5) — not the other way around. `ecs-core` should
   consume a `Motor3` API already proven to compose and invert correctly,
   not be the place that API gets debugged.

   **Milestone before continuing to step 3 — done:** `Vec3`/`Rotor`/
   `Motor3` are implemented and validated: composition, inversion, and
   parent → child transform propagation are covered by both
   `meridian-gac-core`'s test suite and the `gac_validation` example, each
   cross-checked against an independent (non-GA) oracle rather than just
   internal self-consistency. This was the highest-risk layer in the
   workspace (see [ADR 001](adr/001-geometric-algebra-as-spatial-model.md));
   the `Motor3` API held up through this milestone, so the next crates can
   build on it directly.
3. `meridian-memory-core`, `meridian-task-core`, `meridian-platform-core` —
   in parallel, no interdependency.
4. `meridian-resource-core` — typed handles on top of `memory-core`'s
   generic `Handle`, needed before any subsystem below can hold a
   `TextureHandle`/`MeshHandle`/etc.
5. `meridian-ecs-core` — archetype storage, `Transform` as a component.
6. `meridian-compute-driver` → `meridian-compute-runtime` → `meridian-gac-compute`
   — needed before physics or graphics can use compute for batched
   transforms. `gac-compute` depends on both `gac-core` (step 2) and
   `compute-runtime` and is what lets a `Motor3` batch run on CPU-SIMD or
   GPU without `gac-core` or `compute-runtime` depending on each other — see
   [ADR 007](adr/007-batch-transforms-via-compute.md).
7. `meridian-asset-core` — decoders, independent of the above once
   `platform-core` exists.
8. `meridian-graphics-driver` → `meridian-graphics-core`, and
   `meridian-physics-driver` → `meridian-physics-core`, and
   `meridian-audio-driver` → `meridian-audio-core` — in parallel across
   subsystems once their shared dependencies (steps 1-7) exist.
9. `meridian-engine-core` — wires everything into the main loop last, once
   there's something real to schedule.

## Explicitly out of scope for now

- `animation-core`, `particles-core`, `ai-core` — referenced in
  [dependency-rules.md](dependency-rules.md) as future consumers of
  `compute-runtime`, but not part of the current workspace. Add them only when
  there's a concrete subsystem to build, not speculatively.
- Splitting `graphics-driver` into a separately-named RHI crate plus backend
  crates (`vulkan-driver`, etc.) — `graphics-driver` already plays the RHI
  role today (see [graphics-design.md](graphics-design.md)); a rename or
  further split is only worth doing once a second concrete backend actually
  exists to justify it.

## Not yet decided

- Deterministic simulation mode (fixed-point vs. ordered floating point) —
  needed for physics replay/networking, tracked but unspecified.
- Concrete graphics backend(s) beneath `graphics-driver` (Vulkan first,
  per [ADR 005](adr/005-driver-core-separation.md), but not started).
- **`meridian-platform-core`'s `Window` and `DynamicLibrary`** — the
  workspace's first candidates for either an external dependency (`winit`,
  `libloading`) or hand-written unsafe FFI. Decided: hand-written unsafe
  FFI (`dlopen`/`LoadLibrary` for `DynamicLibrary`, per-platform window
  creation for `Window`), not an external crate — the workspace stays at
  zero external dependencies for now. Deliberately deferred: `Window` and
  `DynamicLibrary` aren't needed until `graphics-driver` (step 8), and
  `Time`/`InputState` (implemented, step 3) cover what `resource-core`/
  `ecs-core` (steps 4-5) actually need from `platform-core` in the
  meantime.
