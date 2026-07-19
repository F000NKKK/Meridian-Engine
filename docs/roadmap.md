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
  are real; `BackendCapabilities`/`CpuCapabilities`/`GpuCapabilities`
  (shared by every `*-driver` crate's own capability type) are real;
  `Window` and `DynamicLibrary` are deliberately still stubs — see "Not
  yet decided" below.

Steps 4-6 are real and tested:
- `meridian-resource-core` — `Version::next()`, and `DependencyGraph`
  (tracks declared `ResourceDependency` edges by `Handle`, answers
  `depends_on`/`would_cycle` — topology only, never owns resource data or
  lifetime, per rule 8).
- `meridian-ecs-core` — a real archetype `World`: type-erased SoA columns
  (checked `downcast`, no `unsafe`), archetype migration on `insert`/
  `remove`, single-component `Query`/`QueryMut` (multi-component queries
  deferred — see the crate's module doc for why that's a harder,
  `unsafe`-adjacent problem, not built speculatively).
- `meridian-compute-driver`/`meridian-compute-runtime`/`meridian-gac-compute`
  — a real CPU-parallel dispatch pipeline (`std::thread::scope`, no
  `unsafe`), with `MotorTransformKernel`/`MotorComposeKernel` batching
  `Motor3` composition, cross-checked against direct `gac-core` calls at
  2000+/500+ item batches. No GPU backend — see "Not yet decided" below.

Step 7 (`asset-core`) is real: BMP (uncompressed 24/32-bit), WAV (PCM
16-bit), and a minimal OBJ (positions + triangles) decoder — formats
simple enough to hand-roll without an external crate; PNG/JPEG/glTF need
one, added when a concrete asset needs it, not speculatively.

Step 8's physics half (`physics-driver`/`physics-core`) is real: AABB
broad phase, sphere-sphere narrow phase, an impulse-based constraint
solver (linear *and* angular — see below), semi-implicit Euler
integration. `graphics-driver`/`audio-driver` and the driver-dependent
parts of `graphics-core`/`audio-core` are still scaffolds — blocked on
the GPU backend decision below.

**GA is used in physics where it actually matters, not decoratively**:
angular velocity/torque are `gac-core::Bivector3` (angular quantities
live in so(3), the Lie algebra of rotations — a bivector space, not a
vector space; using `Vec3` for them would be exactly the
vector/bivector conflation GA exists to make explicit, not hide), and
`Integrator` advances orientation via `Bivector3::exp` (a rotor
exponential map composed onto `Motor3`) rather than a naive "add angle"
— the same reason `Transform` is a `Motor3` at all instead of a
quaternion+vector pair (ADR 001). See docs/physics-design.md.

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
8. `meridian-physics-driver` → `meridian-physics-core` — **done** (broad/
   narrow phase, impulse solver, GA-native integration; see "Current
   state" above). `meridian-graphics-driver` → `meridian-graphics-core`
   and `meridian-audio-driver` → `meridian-audio-core` are blocked on the
   GPU backend decision (`wgpu`, see "Not yet decided") for their driver
   halves; the driver-*independent* parts of each `-core` (render graph
   ordering, camera/culling math, spatial audio mixer) don't need to wait
   for that and can start once there's time for them.
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
- **GPU backend beneath `compute-driver`/`graphics-driver`/`physics-driver`
  — decided: `wgpu`, not hand-written per-API FFI.** Reversed from the
  earlier zero-external-dependencies stance: hand-writing Vulkan bindings
  alone (extension loading, swapchain/memory management, synchronization)
  is a multi-month undertaking on its own, "Vulkan *and* DirectX *and*
  Metal by hand" triples that with three independent classes of driver
  bugs, and doing it ourselves wouldn't avoid a dependency so much as
  badly reimplement `wgpu` with more memory-safety risk — `wgpu` is a
  safe Rust API over Vulkan/DX12/Metal/GL, actively maintained, already
  what most Rust engines (Bevy included) use for exactly this. Real cost:
  `wgpu`'s first external dependency, a heavy transitive tree (`wgpu` +
  `naga` + platform GPU bindings), and some async-flavored device/adapter
  acquisition needing `pollster::block_on`-style bridging in an otherwise
  sync codebase — not a reason to pull in a full async runtime (`tokio`)
  for one call. `graphics-driver`'s existing stub shape (`Device`,
  `CommandBuffer`, `Buffer`, `Texture`, `Shader`, `Pipeline`) already maps
  onto `wgpu`'s own vocabulary, so this doesn't force a redesign, just a
  real implementation of what's already stubbed. Not started — deferred
  until `graphics-driver` actually needs it (step 8's remaining half).
  `platform-core`'s `Window`/`DynamicLibrary` are a separate, smaller
  decision (below) and keep their own hand-written-FFI answer; GPU is the
  one deliberate exception to zero-deps, not a reversal of the policy in
  general.
- **`meridian-platform-core`'s `Window` and `DynamicLibrary`** — decided:
  hand-written unsafe FFI (`dlopen`/`LoadLibrary` for `DynamicLibrary`,
  per-platform window creation for `Window`), not an external crate —
  these stay small enough to hand-roll safely, unlike the GPU backend
  above. Deliberately deferred: not needed until `graphics-driver` (step
  8), and `Time`/`InputState`/`BackendCapabilities` (all implemented,
  step 3) cover what every other crate has needed from `platform-core` so
  far.
