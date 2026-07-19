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
broad phase, sphere-sphere/sphere-cuboid/cuboid-cuboid (SAT) narrow phase
(see below for the `Cuboid` collider shape), an impulse-based constraint
solver (linear *and* angular — see below), semi-implicit Euler
integration. `graphics-driver`/`audio-driver` are still scaffolds —
blocked on the GPU backend decision below (audio doesn't strictly need a
GPU, but a real *output device* is the same class of OS-boundary problem
as `Window`, so `audio-driver` is deferred alongside it; `audio-core`
itself doesn't need `audio-driver` to be real to be useful — see next).

**GA is used in physics where it actually matters, not decoratively**:
angular velocity/torque are `gac-core::Bivector3` (angular quantities
live in so(3), the Lie algebra of rotations — a bivector space, not a
vector space; using `Vec3` for them would be exactly the
vector/bivector conflation GA exists to make explicit, not hide), and
`Integrator` advances orientation via `Bivector3::exp` (a rotor
exponential map composed onto `Motor3`) rather than a naive "add angle"
— the same reason `Transform` is a `Motor3` at all instead of a
quaternion+vector pair (ADR 001). See docs/physics-design.md.

`meridian-audio-core` (the driver-independent half of step 8) is real:
`SpeakerLayout` (mono/stereo-headphones/stereo-speakers/5.0/5.1, one VBAP-lite
panning algorithm for all of them — see the crate's module doc for the
`wraps_around` distinction between front-only and real-rear-speaker
layouts, and why front/back correctly collapse to the same centered pan
for stereo but not for 5.0/5.1), `AttenuationModel` (OpenAL's inverse-
clamped-distance model), `Mixer`, and a small `DspGraph`
(`Gain`/`LowPassFilter`). Validated against a listener at the origin
facing `+X` with sources placed front/back/left/right and checked across
every layout (`cargo test -p meridian-audio-core`; human-readable version
via `./build.sh run audio_spatialization`) — including the front/back
ambiguity stereo genuinely can't resolve without HRTF (not implemented;
documented as a real, known limitation, not hidden).

`meridian-graphics-core`'s driver-independent half is real: `Camera`
bridges a `Motor3` world frame into a classical view/projection matrix
(`Motor3::to_mat4` in `gac-core` plus a fixed local-forward-`+X`-to-view-`-Z`
remap, reusing `audio-core`'s listener-forward convention for
cross-subsystem consistency — see docs/graphics-design.md for the full
derivation), `Frustum` does Gribb/Hartmann frustum culling against *any*
`gac-core::Shape` (not just an AABB — see below), and
`RenderGraph::execution_order` derives pass ordering from declared
resource reads/writes (Kahn's algorithm, the same pattern as `task-core`'s
`JobGraph`, applied to resource conflicts instead of explicit
dependencies), rejecting write conflicts and cycles rather than guessing
(`cargo test -p meridian-graphics-core`; human-readable version via
`./build.sh run graphics_validation`). Scene extraction, lighting,
materials-as-shading-inputs, animation and post processing remain scaffolds
— they need an actual GPU resource to shade against, which is blocked on
`graphics-driver`/`wgpu` below.

`meridian-gac-core` also grew a small shared geometric-primitives set:
`Aabb` (moved here after it turned out `physics-core` and `graphics-core`
had each independently written the same one), `Sphere`, `Obb` (oriented
box — `Aabb` is the axis-aligned box variant; a cube is either with equal
extents, not a separate type), `Cone`, `Plane`, and a `Shape` trait (a
support-function interface, the same one GJK/EPA-style algorithms use) that
every one of them implements — so `Plane::contains`/
`ConvexVolume::intersects` (the generalization of `graphics-core`'s
6-plane `Frustum` to any number of planes) are written once, generically,
instead of once per shape pair. All real and tested (`cargo test -p
meridian-gac-core`). `Motor3::transform_vector` (rotation-only action on a
direction, computed as `transform_point(v) - transform_point(ZERO)`, which
cancels translation exactly) was added alongside this, and `Obb` was
changed to store `frame: Motor3` instead of a separate center/orientation
pair — the same pose convention every rigid body in the workspace already
used, so a physics `RigidBody`'s own `frame` builds an `Obb` directly.

`physics-core` grew a second collider shape, `ColliderShape::Cuboid`
(`half_extents` only; orientation comes from the owning `RigidBody`'s
`frame` via `RigidBody::as_obb`) — the point docs/gac-design.md's `Shape`
section anticipated. `NarrowPhase` now handles sphere-sphere,
sphere-cuboid (closest-point-on-box) and cuboid-cuboid (the separating
axis theorem, exactly the "SAT once more shapes exist" this document
already named as the plan), each still producing a single `Contact` point
(no multi-point manifold, matching the existing simplification).
`RigidBody::moment_of_inertia` for `Cuboid` is the average of the box's
three true principal moments, not a full anisotropic tensor — disclosed on
that method's doc comment, forced by `ConstraintSolver` only having a
single scalar `inverse_inertia`. All real and tested, including a
rotated-box SAT case and a full box-settles-on-box-floor integration test
(`cargo test -p meridian-physics-core`). See docs/physics-design.md for
the full breakdown.

**Deterministic simulation mode is real** (see "Not yet decided" below for
the decision this replaces): `meridian-numeric-core::Fixed`, a Q16.16
fixed-point number (`i32`-backed) with exact integer `sqrt` and
CORDIC-based `sin`/`cos`/`atan2` (add/subtract/shift only — no `libm`
call, which is what makes it exactly reproducible across
platforms/compilers, unlike `f32`). `gac-core` now has two GA modules,
`float_ga` (`f32`, the default, re-exported at the crate root so nothing
downstream changed) and `fixed_ga` (`Fixed`, a disclosed one-for-one
duplication of `float_ga`'s structure — see that module's doc comment for
why one generic implementation wasn't used instead: GPUs have no real
`i64` support and are `f32`-native hardware, so a GPU-dispatchable
`Motor3` has to stay floating-point regardless, and GPU parallelism has
its own execution-order nondeterminism on top of that). `physics-core`
gained `deterministic` — `DeterministicBody`/`DeterministicIntegrator`/
`DeterministicConstraintSolver`/`DeterministicBroadPhase`/
`DeterministicNarrowPhase`, mirroring the `f32` pipeline one-for-one,
sphere colliders only for this pass (`Cuboid`/SAT wasn't ported —
tracked as explicit follow-up, not silently dropped). This is
opt-in, not a replacement: `RigidBody` and the rest of `physics-core`'s
default pipeline are untouched; a caller chooses `DeterministicBody`
instead when it needs bit-reproducibility (lockstep networking, replay),
and `DeterministicBody::frame_f32` converts the pose to `gac-core::Motor3`
for handoff to rendering/ECS/audio either way. Proven with an actual
bit-exact reproducibility test — the same scenario run twice via
independent `DeterministicBody` simulations produces identical `Fixed`
bit patterns, not just approximately-equal floats (`cargo test -p
meridian-numeric-core` for `Fixed` itself, `-p meridian-gac-core` for
`fixed_ga` cross-checked against `float_ga` as an oracle, `-p
meridian-physics-core` for the full pipeline and the reproducibility
test; human-readable version via `./build.sh run
determinism_validation`).

Step 9 (`meridian-engine-core`) is real: `SubsystemManager` owns real
`ecs-core`/`physics-core`/`audio-core` instances (the one place in the
workspace allowed to know about every `*-core` at once, per
dependency-rules.md rule 7), `EventSystem` is a type-erased pub/sub
mailbox (`publish`/`drain`, frame-scoped — not a persistent log) that's
the actual mechanism rule 7 exists to enable (subsystems communicating
without depending on each other), and `Runtime::tick` advances physics
then recomputes audio from the physics-updated emitter frames, publishing
a `FrameCompleted` event each frame (`cargo test -p meridian-engine-core`;
human-readable version via `./build.sh run runtime_loop`).
`FrameScheduler` (task-core's `Scheduler` at the engine layer) is real and
tested but not used by `Runtime::tick` itself — physics and audio are
sequentially data-dependent today, not independent branches, so running
them through a job graph would be decorative; see
docs/threading-model.md's `FrameScheduler` section for why, and what makes
it load-bearing later. `graphics-core` isn't wired into `Runtime::tick`
either — rendering has nothing to submit to without a real
`graphics-driver` backend.

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
   state" above). `meridian-audio-core` — **done** (`SpeakerLayout`/
   `Mixer`/`AttenuationModel`/`DspGraph`; see "Current state" above).
   `meridian-graphics-driver` and `meridian-audio-driver` are blocked on
   the GPU/device backend decision (`wgpu`, see "Not yet decided").
   `meridian-graphics-core`'s driver-independent half — **done**: render
   graph pass ordering, `Camera`'s `Motor3` -> view/projection matrix
   bridge, and frustum culling (see "Current state" above). This was the
   harder of the two remaining `-core`s, since it has to bridge
   `gac-core`'s `Motor3` into the classical 4×4 view/projection matrices
   graphics APIs actually need, where `audio-core` could stay entirely in
   GA/`Vec3` terms throughout. Scene extraction, lighting, materials, and
   post-processing are the parts of `graphics-core` still blocked on a
   concrete GPU resource existing (`graphics-driver`/`wgpu`).
9. `meridian-engine-core` — **done** for the driver-independent
   subsystems: `Runtime`/`SubsystemManager`/`EventSystem` wire
   `ecs-core`/`physics-core`/`audio-core` into a real per-frame loop (see
   "Current state" above). `graphics-core` joins once `graphics-driver`
   has a real backend to submit to.

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

- **Deterministic simulation mode — decided: Q16.16 fixed-point
  (`meridian-numeric-core::Fixed`), not ordered floating point.** Ordered
  floating point (careful summation order, no FMA, disabled
  auto-vectorization) was rejected: it fights the compiler/CPU rather
  than sidestepping the problem, and still doesn't fully solve
  cross-`libm` `sin`/`cos`/`sqrt` disagreement, which fixed-point +
  CORDIC avoids by construction (integer add/subtract/shift only). See
  "Current state" above for what's built (`gac-core::fixed_ga`,
  `physics-core::deterministic`) and [ADR
  008](adr/008-fixed-point-determinism.md).
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
- **`meridian-audio-effects` (heavier DSP effects as a separate crate)** —
  decided: not yet, and only split it out when a concrete effect actually
  needs an external dependency. `meridian-audio-core` already owns basic,
  zero-dependency DSP (`DspNode`/`DspGraph`, `Gain`, `LowPassFilter`) and
  should keep owning simple effects that need nothing beyond
  `numeric-core`/`gac-core` — there's no architectural reason to move those
  out. The trigger for a separate crate is the same class of decision as
  `wgpu` below: something like a real convolution reverb, multiband EQ, or
  resampling needs an FFT crate (e.g. `rustfft`), and pulling that into
  `audio-core` would force every consumer of basic spatial audio (including
  `engine-core` and every example) to compile it too. When that concrete
  need exists, `meridian-audio-effects` depends on `meridian-audio-core`
  only (implements its `DspNode` trait, one edge, no new adapter pattern —
  unlike `gac-compute` this isn't avoiding a forbidden edge, just isolating
  an optional heavy dependency) and ships the heavy effects there, keeping
  `audio-core`'s own dependency footprint minimal. Creating the crate before
  that concrete need exists would be exactly the speculative split
  `roadmap.md` already rejects elsewhere (see "Explicitly out of scope").
- **`meridian-platform-core`'s `Window` and `DynamicLibrary`** — decided:
  hand-written unsafe FFI (`dlopen`/`LoadLibrary` for `DynamicLibrary`,
  per-platform window creation for `Window`), not an external crate —
  these stay small enough to hand-roll safely, unlike the GPU backend
  above. Deliberately deferred: not needed until `graphics-driver` (step
  8), and `Time`/`InputState`/`BackendCapabilities` (all implemented,
  step 3) cover what every other crate has needed from `platform-core` so
  far.
