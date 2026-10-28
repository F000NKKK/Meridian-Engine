# Roadmap

## Current state

`meridian-diagnostics`, `meridian-numeric`, and `meridian-gac` have
real implementations: `Vec3`, `Rotor`, `Motor3`, and the underlying 16-blade
`Multivector` geometric product, with a test suite that cross-checks
rotation against an independent Rodrigues-formula oracle and validates the
parent/child transform-hierarchy milestone below (`cargo test -p
meridian-gac`).

`meridian-diagnostics` also owns the workspace's log kernel (`log_error!`..
`log_trace!` macros, stderr + an in-memory ring, the `LogSink` port);
`meridian-diagnostics-fs` is the adapter: a buffered async file sink on a
dedicated tokio thread (Error/Warn/Info file levels, 7-day-default
retention cleanup) and a panic hook writing post-mortem files (message,
location, backtrace, the recent-log tail). Every workspace error type implements `foundation::EngineError`;
drivers and windowed examples log through the sink instead of scattered
`eprintln!`s, and every windowed example installs the crash hook and
recovers a lost/outdated swapchain by reconfiguring the surface instead
of silently skipping frames forever (the "gray window" failure). See
dependency-rules.md rule 0 for why foundation is the open bottom of the
graph.

Step 3 (`memory`/`runtime-threads`/`platform-winit`) is also real and tested:
- `meridian-memory` — generational `ResourcePool<T>` (stale-handle
  detection via generation bump on reuse) and `FrameArena<T>`/
  `PersistentArena<T>` (safe bump-style lists, `reset()` retains capacity).
- `meridian-runtime-threads` — `JobGraph`/`Scheduler`: dependency-ordered
  execution on a persistent worker pool (spawned once with the
  `Scheduler`), panics on a cycle instead of hanging, re-raises a job's panic
  from `run`. Deliberately a shared mutex-guarded ready-queue rather than
  per-worker work-stealing deques — see docs/threading-model.md.
- `meridian-platform-winit` — `Time`/`Clock` and `InputState` (held/
  pressed-this-frame/released-this-frame key state, mouse position+delta)
  are real; `BackendCapabilities`/`CpuCapabilities`/`GpuCapabilities`
  (shared by every `*-driver` crate's own capability type) are real;
  `Window` (a real `winit::window::Window`, see "Not yet decided" below
  for the `winit` decision) and `DynamicLibrary` (real hand-written
  `dlopen`/`LoadLibrary` FFI) are real too — neither is a stub anymore.

Steps 4-6 are real and tested:
- `meridian-resource` — `Version::next()`, and `DependencyGraph`
  (tracks declared `ResourceDependency` edges by `Handle`, answers
  `depends_on`/`would_cycle` — topology only, never owns resource data or
  lifetime, per rule 8).
- `meridian-ecs` — a real archetype `World`: type-erased SoA columns
  (checked `downcast`, no `unsafe`), archetype migration on `insert`/
  `remove`, single-component `Query`/`QueryMut`, and two-to-four-component
  `World::query_many`/`query_many_mut` (safe: `HashMap::get_disjoint_mut`
  hands out the disjoint columns, see docs/ecs-design.md).
- `meridian-compute-wgpu`/`meridian-compute-app`/`meridian-gac-wgpu`
  — a real CPU-parallel dispatch pipeline (`std::thread::scope`, no
  `unsafe`), with `MotorTransformKernel`/`MotorComposeKernel` batching
  `Motor3` composition, cross-checked against direct `gac` calls at
  2000+/500+ item batches. `compute-wgpu` also has a real GPU backend
  now (`GpuComputeDevice`, shared `wgpu` mechanics with `rendering-wgpu`
  via `meridian-gpu-wgpu`), reachable through
  `compute-app::ComputeContext::with_gpu` — see "Not yet decided"
  below ([ADR 011](adr/011-shared-gpu-driver-crate.md)). `MotorTransformKernel`/
  `MotorComposeKernel` themselves still only dispatch through
  `compute-app`'s CPU `parallel_for` path — wiring their own
  GPU-dispatching variant is separate follow-up. What *does* dispatch on
  GPU today: `gac-wgpu::fixed_wgsl`, real Q16.16 `Fixed`-point
  `+`/`-`/`*`/`/` running as WGSL compute shaders (32x32->64 multiply/
  divide emulated from 32-bit words — WGSL has no native `i64`), proven
  bit-for-bit identical to the CPU `Fixed` implementation across ~1000
  operand pairs per operation, not just numerically close (`cargo test -p
  meridian-gac-wgpu fixed_wgsl`) — `sqrt` now included (Newton's
  method ported from `isqrt_u64`, same bit-exactness proof), and so is
  CORDIC `sin_cos`/`atan2`: unlike the arithmetic ops, CORDIC needs no
  64-bit emulation (only shifts/adds on a single `i32`, and WGSL's `>>`
  on `i32` is already the same arithmetic sign-extending shift Rust
  uses), so it's a near-literal translation of
  `numeric::fixed`'s `cordic_rotate`/`cordic_vector_angle`, with the
  `atan` table and gain/pi constants precomputed on the Rust side and
  embedded as WGSL literals. Same bit-exactness bar, proven against the
  CPU `Fixed::sin_cos`/`atan2` across a full-circle angle sweep and every
  `atan2` quadrant (`cargo test -p meridian-gac-wgpu fixed_wgsl`).

  A real domain kernel now dispatches through it: `meridian-physics-wgpu`
  (a new `meridian-<domain>-compute` adapter crate per rule 11, mirroring
  `gac-wgpu`'s shape one layer up) has a GPU soft-body (mass-spring)
  integration kernel, both flavors — `float` (numerically close to
  `SoftBodyIntegrator::step`, not bit-exact — GPU float summation order
  can differ from the CPU's) and `Fixed` (proven bit-exact against
  `FixedSoftBodyIntegrator::step` across 60 steps, `cargo test -p
  meridian-physics-wgpu fixed::`). The reformulation from "iterate
  springs, scatter into both endpoints" (the CPU version, a data race if
  parallelized directly) to "iterate particles, gather from adjacent
  springs" (the GPU version) turned out to have a real bit-exactness trap
  for the `Fixed` flavor: `Fixed::mul`'s `>>16` truncation rounds toward
  negative infinity, so recomputing a spring's direction independently
  from each endpoint's own sign-flipped position delta and then
  multiplying is *not* exactly the negation of the other endpoint's
  result (they can differ by one raw bit whenever the discarded low bits
  are nonzero — vector subtraction/negation are exact, but multiplication
  isn't, under this rounding rule). The fix — both endpoints compute the
  same canonical direction and spring force, and only the non-canonical
  endpoint negates the *finished* force vector (exact) rather than
  re-deriving it from a negated direction (not exact) — is documented in
  `meridian-physics-wgpu::generic`'s module doc as a worked example of
  why "provably symmetric on paper" isn't the same as "bit-exact in
  fixed-point," for the next time this class of reformulation comes up.

  `compute-app` also has a real hybrid CPU+GPU dispatch mechanism now:
  `HybridKernel` (`run_cpu`/`run_gpu`, one method per backend, each
  covering an index range) plus `ComputeContext::run_hybrid`, which
  splits `count` work items between both per a `BackendSplit` policy
  (`Ratio(0.5)` for an even split, `CpuOnly`/`GpuOnly` for either
  extreme) and runs both halves **concurrently** — the CPU half on
  `tokio::task::spawn_blocking`'s thread pool, the GPU half awaited
  directly, real overlap rather than sequential CPU-then-GPU. Proven by a
  real doubling kernel (CPU doubles in Rust, GPU doubles via WGSL, both
  write into the same shared output buffer by index) that checks every
  index landed correct regardless of which backend processed it, plus a
  no-GPU-backend fallback test (`cargo test -p meridian-compute-app`).
  This is deliberately *not* an automatic "run this same code on either
  backend" switch — there's no such thing for an arbitrary Rust closure,
  since it has no WGSL equivalent a compiler can derive; a kernel author
  has to write both implementations by hand, same as `fixed_wgsl` does.
  Buffer and bind-group reuse is built for the soft-body kernels
  (`compute-wgpu::{ScratchSlot, GenerationCache, ScratchPool}`): buffers are
  sized once and rewritten per call, the bind group is rebuilt only when a
  buffer grew. Other kernels still allocate per call until a per-frame caller
  needs them converted.

`physics-domain`'s rigid-body engine (`BroadPhase`/`NarrowPhase`/
`ConstraintSolver`/`Integrator`) was, until now, only ever driven by a
plain CPU `for` loop — the "batching them through `compute-app` is
additive later" note earlier in this document. `Integrator`'s half of
that is real now: `meridian-physics-wgpu::rigid_body::RigidBodyIntegratorKernel`
batches `Integrator::step` across many independent bodies through one
`ComputeKernel::dispatch` call instead of a loop, the same shape as
`gac-wgpu::MotorTransformKernel`. Unlike this crate's soft-body
kernels (`float`/`fixed`, WGSL, float/fixed-split because the GPU
dispatch itself forces it), `RigidBodyIntegratorKernel` is generic over
`GaFlavor` and dispatches through `ComputeContext::parallel_for`'s CPU
backend only — `Integrator::step` has no GPU-dispatch constraint of its
own (same reasoning as `physics-domain`'s own engine being written once
and generic, not float/fixed-duplicated), so there's nothing to
WGSL-port yet, and CPU batching is the real, useful step this phase
needed. Proven to match calling `Integrator::step` directly, including
its static-body (`mass <= 0`) skip and a batch above the CPU
parallel-dispatch threshold (`cargo test -p meridian-physics-wgpu
rigid_body`). `NarrowPhase::test_pair` is batched too, the same way:
`meridian-physics-wgpu::narrow_phase::NarrowPhaseTestPairKernel`
tests many candidate pairs (typically
`BroadPhase::find_candidate_pairs`'s output) for exact overlap through
one dispatch, mirroring `gac-wgpu::MotorComposeKernel`'s
independent-pairs shape — `test_pair` is one pair in, one
`Option<Contact<F>>` out, a fixed 1:1 shape, deliberately not
`NarrowPhase::generate_contacts` (a box-box pair expands to a *variable*
number of manifold points, which doesn't fit a fixed-size-per-item
kernel slot). Same proof shape as `RigidBodyIntegratorKernel`: matches
calling `test_pair` directly pair-for-pair, including which pairs come
back `None`, across a batch above the parallel-dispatch threshold
(`cargo test -p meridian-physics-wgpu narrow_phase`).
`BroadPhase::find_candidate_pairs` is batched too, the same fixed
one-output-per-item shape:
`meridian-physics-wgpu::broad_phase::BroadPhasePairsKernel` tests
every candidate pair's AABB overlap in parallel (via `RigidBody::aabb`
— a new public method exposing the same bound `find_candidate_pairs`
already computed internally, reused rather than re-derived) and returns
the exact same surviving pairs in the exact same order.

`ConstraintSolver` is batched too, closing out the rigid-body pipeline
— the hardest of the four, since `resolve_velocity`/
`apply_positional_correction` each mutate *two* bodies, so naively
running distinct contacts in parallel races whenever they share a body
(the common case for a multi-point manifold) and would silently turn
the solver's deliberately Gauss-Seidel sequential relaxation into
Jacobi instead, a real behavior change, not just a speed one.
`meridian-physics-wgpu::constraint_solver::ConstraintSolverBatchKernel`
solves this with graph coloring (`color_contacts`): contacts are
grouped so no two contacts sharing a body land in the same group, each
group's contacts run concurrently (real per-body `Mutex` locking, not
one shared lock), and groups run in sequence — reproducing the
sequential solver's exact dependency order. Proven bit-for-bit
identical to the sequential `resolve_velocity`/
`apply_positional_correction` loop on a real multi-point box-on-floor
manifold, plus a 600-tick settling regression test matching
`runtime-app`'s own sequential-solver test (`cargo test -p
meridian-physics-wgpu constraint_solver`). This closes every item
`docs/physics-design.md`'s Compute section named as open — see that
document for the full breakdown. `NarrowPhase::generate_contacts` is
batched too — earlier than the other four, in fact:
`meridian-physics-wgpu::narrow_phase::GenerateContactsKernel`
handles its variable-per-pair manifold output (a box-box pair expands
to up to `MAX_CONTACTS_PER_PAIR` points) with a fixed-size-per-pair
output slot (`[Option<Contact<F>>; MAX_CONTACTS_PER_PAIR]`, flattened
back to pair order for the result), rather than a true dynamic
prefix-sum — the same `MAX_LIGHTS`-style fixed-capacity-array approach
`rendering-domain::submission` uses for its own per-frame uniform array,
truncating (with a logged warning) only if a pair's manifold ever
exceeds the cap, which `physics-domain::collision::face_manifold`'s own
`.take(4)` currently guarantees it never does. So all five rigid-body
pipeline steps — `Integrator`, `BroadPhase::find_candidate_pairs`,
`NarrowPhase::test_pair`, `NarrowPhase::generate_contacts` and
`ConstraintSolver` — are batched now; a true dynamic prefix-sum (for a
manifold cap that might someday grow past a small fixed bound) remains
a hypothetical future refinement, not an open gap.

Step 7 (`asset-domain`) is real: BMP (uncompressed 24/32-bit), WAV (PCM
16-bit), and a minimal OBJ (positions + triangles) decoder — formats
simple enough to hand-roll without an external crate. Compressed audio
is real too: MP3/OGG-Vorbis/FLAC via `symphonia` and Opus via the
reference `libopus` in the same symphonia pipeline, with formats
identified by leading magic bytes (`AudioFormat::detect` /
`AnyAudioDecoder`), never by file extension — the concrete asset that
justified the external codec crates is `examples/assets/audio/`; see
[ADR 013](adr/013-compressed-audio-codecs.md). Audio decoding lives in
the `audio/` barrel module (`wav`/`compressed`/`streaming`),
and `open_audio(bytes, &DecodeStrategy)` is its thin loading interface:
full decode vs. incremental streaming resolved entirely by
configuration (force either, or `Auto` by estimated decoded size), with
`StreamingAudioDecoder` shaped as a standard `Iterator` — see
[ADR 014](adr/014-streaming-audio-decode.md), including why no generic
"asset stream" trait is invented before a second streamed asset type
(video would be a sibling `video/` module) exists. PNG/JPEG/glTF stay on
the same when-a-concrete-asset-needs-it trigger.

Step 8's physics half (`physics-driver`/`physics-domain`) is real: AABB
broad phase, sphere-sphere/sphere-cuboid/cuboid-cuboid (SAT) narrow phase
(see below for the `Cuboid` collider shape), an impulse-based constraint
solver (linear *and* angular — see below), semi-implicit Euler
integration. `rendering-wgpu` is real now, headless and windowed — see "Not yet
decided" below for the `wgpu` details. `audio-cpal` is real, backed by
`cpal` (see [ADR 012](adr/012-audio-output-via-cpal.md)): async device
enumeration per ADR 009, and an `AudioStream` that bridges cpal's
real-time callback model into a push model — the game thread pushes
interleaved `f32` samples into a bounded ring buffer, the hardware
callback drains it, blocking-on-full is the backpressure. Only `f32`
output for now; `audio-domain` never appears in its dependency graph — see
next.

**GA is used in physics where it actually matters, not decoratively**:
angular velocity/torque are `gac::Bivector3` (angular quantities
live in so(3), the Lie algebra of rotations — a bivector space, not a
vector space; using `Vec3` for them would be exactly the
vector/bivector conflation GA exists to make explicit, not hide), and
`Integrator` advances orientation via `Bivector3::exp` (a rotor
exponential map composed onto `Motor3`) rather than a naive "add angle"
— the same reason `Transform` is a `Motor3` at all instead of a
quaternion+vector pair (ADR 001). See docs/physics-design.md.

`ConstraintSolver` resting-contact stabilization is also real now, after
a jitter/clipping bug found via the `physic_figures` example (multi-point
box/pyramid contacts visibly bouncing and sinking into the floor at
rest): `resolve` used to apply both the velocity impulse and the
positional (Baumgarte-style) correction in one call, so calling it more
than once per tick — the normal relaxation-pass pattern for multi-point
manifolds — over-applied the positional push each pass. `resolve` is now
split into `resolve_velocity` (safe to call once per relaxation pass)
and `apply_positional_correction` (call exactly once per tick, after the
relaxation loop), so callers control the pass count without compounding
position correction. Separately, gravity re-applied every tick against a
nonzero restitution produced a never-ending, shrinking-but-nonzero
bounce on resting contacts (imperceptible for a single sphere, visible
as continuous up/down micro-jitter on multi-point manifolds);
`ConstraintSolver` gained a `restitution_velocity_threshold` field
(default 0.5 world-units/second, opt-in via
`with_restitution_velocity_threshold`) that suppresses restitution below
that closing speed while leaving real-impact behaviour (well above the
threshold) unchanged, and `physic_figures` additionally sets its solver
restitution to `0.0` so landings are fully inelastic. Covered by a
settled-body regression test asserting height stability (no
bounce-then-clip) across many ticks (`cargo test -p
meridian-physics-domain`, the `drop`/settling tests). See
docs/physics-design.md's "Resting-contact stabilization" section for the
`resolve_velocity`/`apply_positional_correction` split and the
restitution threshold in detail.

`meridian-audio-domain` (the driver-independent half of step 8) is real:
`SpeakerLayout` (mono/stereo-headphones/stereo-speakers/5.0/5.1, one VBAP-lite
panning algorithm for all of them — see the crate's module doc for the
`wraps_around` distinction between front-only and real-rear-speaker
layouts, and why front/back correctly collapse to the same centered pan
for stereo but not for 5.0/5.1), `AttenuationModel` (OpenAL's inverse-
clamped-distance model), `Mixer`, and a small `DspGraph`
(`Gain`/`LowPassFilter`). Validated against a listener at the origin
facing `+X` with sources placed front/back/left/right and checked across
every layout (`cargo test -p meridian-audio-domain`) — including the front/back
ambiguity stereo genuinely can't resolve without HRTF (not implemented;
documented as a real, known limitation, not hidden). On top of the
`Mixer` path, `audio-domain::effects` (one effect per file:
`Gain`/`LowPassFilter`/`DspGraph`/`BinauralRenderer`) adds the
physically-motivated headphone spatializer: ITD via fractional-sample
delay lines, azimuth-dependent head-shadow low-pass on the far ear, rear
damping (no front-fold on this path), and per-block parameter ramps that
eliminate zipper/crackle when the listener moves — deliberately CPU-side:
per block it's a few thousand samples of inherently serial IIR work,
orders of magnitude below GPU dispatch+readback cost; batched
FFT/HRTF-convolution over many sources is the future
`audio-effects`/`*-compute` trigger. With `audio-cpal`
real, `audio-domain` also owns the output bridge: `Mixer::render_interleaved`
(block rendering to interleaved speaker-order samples) and `AudioOutput`
(opens the default device with one stream channel per speaker) — audible
end-to-end via the `magic_figures` example (see below).

`meridian-rendering-domain`'s driver-independent half is real: `Camera`
bridges a `Motor3` world frame into a classical view/projection matrix
(`Motor3::to_mat4` in `gac` plus a fixed local-forward-`+X`-to-view-`-Z`
remap, reusing `audio-domain`'s listener-forward convention for
cross-subsystem consistency — see docs/graphics-design.md for the full
derivation), `Frustum` does Gribb/Hartmann frustum culling against *any*
`gac::Shape` (not just an AABB — see below), and
`RenderGraph::execution_order` derives pass ordering from declared
resource reads/writes (Kahn's algorithm, the same pattern as `runtime-threads`'s
`JobGraph`, applied to resource conflicts instead of explicit
dependencies), rejecting write conflicts and cycles rather than guessing
(`cargo test -p meridian-rendering-domain`). Scene extraction, lighting,
materials-as-shading-inputs and post processing (bloom) are real now, and
`rendering-wgpu` presents to a window (see "Current state" below); only
animation is still a scaffold.

`meridian-gac` also grew a small shared geometric-primitives set:
`Aabb` (moved here after it turned out `physics-domain` and `rendering-domain`
had each independently written the same one), `Sphere`, `Obb` (oriented
box — `Aabb` is the axis-aligned box variant; a cube is either with equal
extents, not a separate type), `Cone`, `Plane`, and a `Shape` trait (a
support-function interface, the same one GJK/EPA-style algorithms use) that
every one of them implements — so `Plane::contains`/
`ConvexVolume::intersects` (the generalization of `rendering-domain`'s
6-plane `Frustum` to any number of planes) are written once, generically,
instead of once per shape pair. All real and tested (`cargo test -p
meridian-gac`). `Motor3::transform_vector` (rotation-only action on a
direction, computed as `transform_point(v) - transform_point(ZERO)`, which
cancels translation exactly) was added alongside this, and `Obb` was
changed to store `frame: Motor3` instead of a separate center/orientation
pair — the same pose convention every rigid body in the workspace already
used, so a physics `RigidBody`'s own `frame` builds an `Obb` directly.

`physics-domain` grew a second collider shape, `ColliderShape::Cuboid`
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
(`cargo test -p meridian-physics-domain`). See docs/physics-design.md for
the full breakdown.

**Deterministic simulation mode is real** (see "Not yet decided" below for
the decision this replaces): `meridian-numeric::Fixed`, a Q16.16
fixed-point number (`i32`-backed) with exact integer `sqrt` and
CORDIC-based `sin`/`cos`/`atan2` (add/subtract/shift only — no `libm`
call, which is what makes it exactly reproducible across
platforms/compilers, unlike `f32`) — split into its own file
(`numeric::fixed`) alongside the existing `f32` flavor
(`numeric::float`), both re-exported at the crate root. `gac`
grew a `fixed_ga` module (`Fixed`-backed `Multivector`/`Vec3`/`Bivector3`/
`Rotor`/`Motor3`) alongside the existing `float_ga` (`f32`, the default,
re-exported at the crate root so nothing downstream changed) — this pair
*is* a deliberate, disclosed duplication (see `fixed_ga`'s doc comment for
why: GPUs have no real `i64` support and are `f32`-native hardware, so a
GPU-dispatchable `Motor3` has to stay floating-point regardless, and GPU
parallelism has its own execution-order nondeterminism on top of that).
The pure-integer `blade` bitmask module (scalar-type-independent) lives
once at the crate root, shared by both. Converting between flavors is a
named method (`Vec3::to_fixed_lossy`/`FixedVec3::to_float_lossy`, same
pair for `Bivector3`/`Rotor`/`Motor3`), not `From`/`Into` — deliberately,
so a precision/determinism-changing cast never looks free at the call
site; `Vec3`/`FixedVec3` also support direct mixed-type `+`/`-`/`*` built
on those same named conversions. `gac-wgpu` grew `Fixed` counterparts
too (`FixedMotorTransformKernel`/`FixedMotorComposeKernel`), CPU-dispatch
only in practice (no GPU backend exists yet to actually dispatch to, not
a type-level restriction — see that crate's doc comment).

Everything *besides* the algebra itself — `Aabb`/`Sphere`/`Obb`/`Cone`/
`Plane`/`Shape`/`ConvexVolume`/`Projection`/`Frame` in `gac`, and the
entire `physics-domain` engine (`RigidBody`/`ColliderShape`/`Contact`/
`BroadPhase`/`NarrowPhase`/`ConstraintSolver`/`Integrator`) — has no
GPU-dispatch constraint of its own, so it's written **once**, generic
over a `GaFlavor` trait (`gac::generic::GaFlavor`, bundling
`ScalarLike`/`VectorLike`/`BivectorLike`/`RotorLike`/`MotorLike`), not
hand-duplicated per scalar flavor. `gac::float_ga`/`fixed_ga` and
`physics-domain::float`/`fixed` each expose thin `FloatFlavor`/`FixedFlavor`
type aliases over that one generic implementation (`float`'s aliases
re-exported at each crate's root, so existing unparameterized call sites
are unaffected). This was a correction made partway through the original
determinism work: `physics-domain::deterministic` initially hand-duplicated
`RigidBody`/`Integrator`/etc. as `DeterministicBody`/
`DeterministicIntegrator`/etc. (sphere colliders only, since porting SAT
by hand was deferred as risky/duplicative) — caught on review as
duplicating logic with no GPU-dispatch justification, and replaced by the
generic engine described above. A direct benefit of that correction:
because `physics-domain`'s engine is now generic rather than duplicated,
sphere-sphere, sphere-cuboid *and* cuboid-cuboid (SAT) narrow phase all
work for the `Fixed` flavor for free — there's no "sphere only" scope
limit left to track as follow-up. `fixed::RigidBody`/etc. are opt-in, not
a replacement: `float::RigidBody` (aliased at the crate root) and the
rest of `physics-domain`'s default pipeline are untouched; a caller chooses
`physics-domain::fixed::RigidBody` instead when it needs bit-reproducibility
(lockstep networking, replay), and `FixedMotor3::to_float_lossy` (called
on its `frame`) converts the pose to `gac::Motor3` for handoff to
rendering/ECS/audio either way. Proven with an actual bit-exact
reproducibility test — the same scenario run twice via independent
`fixed::RigidBody` simulations produces identical `Fixed` bit patterns,
not just approximately-equal floats (`cargo test -p meridian-numeric`
for `Fixed` itself, `-p meridian-gac` for `fixed_ga` cross-checked
against `float_ga` as an oracle, `-p meridian-gac-wgpu` for the `Fixed`
kernels, `-p meridian-physics-domain` for the full pipeline and the
reproducibility test; human-readable version via `./build.sh run
determinism_validation`). See
[ADR 008](adr/008-fixed-point-determinism.md) for the full decision,
including the layering correction (primitives belong in `gac`, not
`physics-domain`) and the later genericization correction, both made
partway through this work.

Step 9 (`meridian-runtime-app`) is real: `SubsystemManager` owns real
`ecs`/`physics-domain`/`audio-domain` instances (the one place in the
workspace allowed to know about every `*-core` at once, per
dependency-rules.md rule 7), `EventSystem` is a type-erased pub/sub
mailbox (`publish`/`drain`, frame-scoped — not a persistent log) that's
the actual mechanism rule 7 exists to enable (subsystems communicating
without depending on each other), and `Runtime::tick`/`Runtime::tick_fixed`
advance physics then recompute audio from the physics-updated emitter
frames, publishing a `FrameCompleted` event each frame (`cargo test -p
meridian-runtime-app`). `tick_fixed(dt)` bypasses the internal
wall-clock entirely (steps by exactly `dt`, tracks its own
`total_seconds`) — added because `physics-domain`'s solver is only
validated at a constant timestep, so a real application's accumulator
loop needs to call *something* with a fixed `dt`, not `tick`'s
variable wall-clock-driven one. `FrameScheduler` (runtime-threads's
`Scheduler` at the engine layer) is real and tested but not used by
`Runtime::tick`/`tick_fixed` themselves — physics and audio are
sequentially data-dependent today, not independent branches, so running
them through a job graph would be decorative; see
docs/threading-model.md's `FrameScheduler` section for why, and what makes
it load-bearing later. `rendering-domain`'s scene/material/lighting/
submission layers are real now too (see the `submission` entry above:
`SceneRenderer`, real `Blinn`-Phong lit draws, bloom) — and
presenting a frame is wired into `Runtime::tick` through
`meridian_sdk::RenderStage`, a `Stage` over the `FrameRecorder` port
([ADR 029](adr/029-render-stage-presents-through-the-frame-recorder-port.md)),
so `runtime-app` still never depends on `rendering-wgpu`.
`physic_figures` registers it after its physics stage.

**`Runtime`/`SubsystemManager` are now proven end-to-end, not just
tested in isolation.** `examples/physic_figures` drives its physics
stepping through a real `Runtime` (`Runtime::tick_fixed` inside a
fixed-timestep accumulator, replacing the `meridian_sdk::pipeline::Pipeline`
it used before that) — the first real application in this workspace to
actually construct and step a `Runtime`. `examples/magic_figures` drives its audio
through a `Runtime` too: an `AudioStage` renders an `AudioScene` (looping
`Voice`s, `BinauralRenderer`, `Declicker`) into the stream's `StreamSink`,
while the example only sets the listener and voice emitters each frame. `meridian_sdk::pipeline::Pipeline`
itself remains real, tested (`cargo test -p meridian-sdk`), and
available for exactly that kind of custom multi-stage composition — it
just isn't `physic_figures`' driver anymore, since a single fixed
physics step is `Runtime`'s job now that `Runtime` has the API for it.

**Shadow mapping and hemisphere ambient are real now, not future
work.** `Scene3d::ambient_ground`/`ambient_sky` replace the old flat
`ambient` fill — a two-color hemisphere gradient lerped by each
fragment's world-normal.y, a disclosed lightweight step toward real
image-based lighting (a full convolved-cubemap IBL pipeline is still
future work, just no longer the *only* option). The first
`Light::Directional` in a scene's `lights` now casts a real shadow: a
depth-only pass (`SceneRenderer::render_shadow_pass`, `rendering-wgpu`'s
new `create_shadow_map_texture`/`create_shadow_pipeline`/
`begin_shadow_pass`) renders occluder depth into a
`Depth32Float` shadow map from the light's own orthographic view (see
`directional_shadow_camera` — a fixed volume, since replaced by four
frustum-fitted cascades in a 2 x 2 atlas, ADR 037), and
both lit shader flavors (`lit_shader_wgsl`/`lit_textured_shader_wgsl`)
sample it via `textureSampleCompare` (hardware PCF via a comparison
sampler) with a small constant bias, combined with front-face culling
in the shadow pass itself, against "shadow acne." Proven in both
examples (`physic_figures`/`magic_figures`, via
`meridian_examples::render::render_frame`) running many frames without
a `wgpu` validation error — `directional_shadow_camera`'s own matrix
math (depth ordering, scene-center centering) is unit-tested in
isolation (`cargo test -p meridian-rendering-domain`), the same
"unverifiable-without-a-window, so windowed examples are the proof"
convention this crate's own test module already documents for
`SceneRenderer` itself. Radiance `.hdr` panoramas and cube maps load as environments and can
be the visible backdrop (ADR 036, ADR 038, ADR 039). Shadows are cascaded, culled per cascade and blended at the seams (ADR 037, ADR 040), and the first point light casts its own (ADR 041). **Not yet done:** soft shadows and several shadowing point lights (HDR and tone mapping are done, ADR 034), not a blanket
"every other crate is a scaffold." Audio is wired end-to-end:
`audio-domain::AudioOutput` + `Mixer::render_interleaved` bridge mixed
samples into `audio-cpal`'s real stream (the declared
core→own-driver edge, used for the first time), and — for
`physic_figures`, which has no audio needs of its own — through
`runtime-app::Runtime`/`SubsystemManager` for real, per the entry
above. `magic_figures` is the full audio+graphics loop: a windowed
scene with three orbiting shapes (sphere, cube, pyramid), each emitting
its own decoded music track (a different container/codec format per
shape) while the `FlyCamera` pose doubles as the shared `audio-domain`
listener (same local-forward-`+X` `Motor3` convention), so panning and
distance attenuation track the camera live; `physic_figures` runs the
same shapes as real `physics-domain` rigid bodies settling onto a
textured floor, sharing `meridian_sdk::scene::GraphicsSession` with
`magic_figures`. Every crate not named above
has a real, tested implementation; see each crate's own section above
for specifics. This staged order is intentional — see "Why implementation
is deliberately last" below.

## Craftbook migration

The workspace is moving to the Craftbook standard in waves (plan and
measurements: Craftbook's `adoption/meridian-engine.md`; decisions:
[ADR 019](adr/019-ports-in-domain-drivers-as-adapters.md),
[ADR 020](adr/020-craftbook-adoption.md)). Progress is measured by the
committed baselines, which only shrink: `.craftcheck-baseline`,
`.craftcheck-deps-baseline`, `.clippy-backlog`, and `PENDING` in
`scripts/check_dependency_rules.py`.

- Wave 0 (gates): lint set, `craftbook.toml`, baselines, CI workflow,
  `hw-tests` feature, glossary and context map — done.
- Waves 1–3 (hygiene, wiring-only `lib.rs`, splitting the largest files,
  long and wide functions, `proptest` for `numeric`/`gac`,
  `criterion` benches for the four `perf` crates) — done, except the
  files and functions that wave 4 rewrote.
- Wave 4 (float/fixed layout: `gac`, `physics-domain`, `soft_body`,
  `physics-wgpu` as one directory per concept) — done. Waves 5–7
  (inverting the driver edges, runtime/composition root, crate renames)
  follow.

## Why implementation is deliberately last

The riskiest failure mode at this project's current size isn't technical
complexity — it's layers slowly bleeding into each other (a rendering
concept leaking into `ecs`, a global manager reappearing in
`asset-domain`, a driver crate quietly depending on its own core). Once real
code exists, every such violation costs a refactor. Right now it costs a
one-line correction to [dependency-rules.md](dependency-rules.md). The
priority before writing implementations is keeping that document and the
[ADRs](adr/) accurate and complete.

## Suggested implementation order

1. `meridian-diagnostics`, `meridian-numeric` — scalar/SIMD groundwork
   everything else depends on.
2. `meridian-gac` — motors, rotors, transforms. Nothing above this
   layer can be meaningfully tested without it. Build it bottom-up and
   validate before moving on: `Vec3` → `Rotor3` → `Motor3` → a parent/child
   transform-hierarchy composition test, *then* wire `Transform` into
   `ecs` (step 5) — not the other way around. `ecs` should
   consume a `Motor3` API already proven to compose and invert correctly,
   not be the place that API gets debugged.

   **Milestone before continuing to step 3 — done:** `Vec3`/`Rotor`/
   `Motor3` are implemented and validated: composition, inversion, and
   parent → child transform propagation are covered by both
   `meridian-gac`'s test suite and the `gac_validation` example, each
   cross-checked against an independent (non-GA) oracle rather than just
   internal self-consistency. This was the highest-risk layer in the
   workspace (see [ADR 001](adr/001-geometric-algebra-as-spatial-model.md));
   the `Motor3` API held up through this milestone, so the next crates can
   build on it directly.
3. `meridian-memory`, `meridian-runtime-threads`, `meridian-platform-winit` —
   in parallel, no interdependency.
4. `meridian-resource` — typed handles on top of `memory`'s
   generic `Handle`, needed before any subsystem below can hold a
   `TextureHandle`/`MeshHandle`/etc.
5. `meridian-ecs` — archetype storage, `Transform` as a component.
6. `meridian-compute-wgpu` → `meridian-compute-app` → `meridian-gac-wgpu`
   — needed before physics or graphics can use compute for batched
   transforms. `gac-wgpu` depends on both `gac` (step 2) and
   `compute-app` and is what lets a `Motor3` batch run on CPU-SIMD or
   GPU without `gac` or `compute-app` depending on each other — see
   [ADR 007](adr/007-batch-transforms-via-compute.md).
7. `meridian-asset-domain` — decoders, independent of the above once
   `platform-winit` exists.
8. `meridian-physics-driver` → `meridian-physics-domain` — **done** (broad/
   narrow phase, impulse solver, GA-native integration; see "Current
   state" above). `meridian-audio-domain` — **done** (`SpeakerLayout`/
   `Mixer`/`AttenuationModel`/`DspGraph`; see "Current state" above).
   `meridian-rendering-wgpu` — **done**: real `wgpu`
   `Device`/`Buffer`/`Texture`/`Shader`/compute `Pipeline`/`CommandBuffer`,
   headless and windowed (`Device::new_windowed`, a real swapchain —
   see "Not yet decided" below). `meridian-audio-cpal` — **done**,
   backed by `cpal` (see [ADR 012](adr/012-audio-output-via-cpal.md)
   and "Current state" above).
   `meridian-rendering-domain`'s driver-independent half — **done**: render
   graph pass ordering, `Camera`'s `Motor3` -> view/projection matrix
   bridge, and frustum culling (see "Current state" above). This was the
   harder of the two remaining `-core`s, since it has to bridge
   `gac`'s `Motor3` into the classical 4×4 view/projection matrices
   graphics APIs actually need, where `audio-domain` could stay entirely in
   GA/`Vec3` terms throughout. Scene extraction, lighting, materials, and
   post-processing are the parts of `rendering-domain` still blocked on a
   window/swapchain surface and a mesh/material vocabulary, neither of
   which exist yet.
9. `meridian-runtime-app` — **done** for the driver-independent
   subsystems: `Runtime`/`SubsystemManager`/`EventSystem` wire
   `ecs`/`physics-domain`/`audio-domain` into a real per-frame loop (see
   "Current state" above). `rendering-domain` joins once a window/swapchain
   exists for `rendering-wgpu` to submit frames to.
10. `meridian-sdk` — **done** for its current scope: a single
    application-facing re-export crate (`examples/` depends on it
    alone), its own composable `pipeline::Pipeline` (job-graph-based,
    fine-grained locking over `PhysicsSubsystem`/`AudioSubsystem`,
    replacing the earlier hand-rolled per-example physics stepping —
    proven end-to-end in `physic_figures`), resource loading
    (`assets::AssetCache::load_texture`/`load_mesh_obj`,
    `assets::load_audio_track` — all decode/cache/downmix logic lives
    here, not duplicated per example), and the extensible scene DSL
    (`dsl` module — `#[dsl_tag(name = "...")]` lets a game register its
    own tags rather than picking from a fixed schema; see
    [ADR 015](adr/015-extensible-scene-dsl.md) and `meridian_sdk::dsl`'s
    own module doc). **Both examples now build their scenes from a real
    `.meui` file** (`examples/assets/scenes/{physic_figures,magic_figures}.meui`,
    Meridian Engine Language — today just the tag/attribute DSL; the
    planned direction is WPF-style code-behind with compiled bindings,
    see [ADR 017](adr/017-meui-markup-and-code-behind.md)),
    parsed via `dsl::build_scene` — `physic_figures` uses only the
    SDK's built-in tags (`Entity`/`Transform`/`Mesh`/`Material`/
    `RigidBody`); `magic_figures` additionally defines its own
    `Glow`/`Orbit`/`Audio` tags with the same `#[dsl_tag]` macro,
    demonstrating the actual extension mechanism end-to-end (a
    game-specific tag composing with SDK built-ins in one document).
    Per-frame *behavior* (orbit/spin motion, the fixed-timestep physics
    accumulator) stays genuine Rust — the DSL describes composition
    only, per its documented scope. `physic_figures` now drives its
    physics through `meridian-runtime-app::Runtime`/`SubsystemManager`
    for real (`Runtime::tick_fixed`, see that crate's "Current state"
    entry above) — no longer an open item. Real shadow mapping and
    hemisphere ambient lighting in `rendering-domain` are also done now
    (see the "shadow mapping and hemisphere ambient" entry above) — no
    longer an open item either. **Not yet done:** an app-shell schema
    (window title, logging, crash-report config) was considered and
    deliberately deferred; `.meui` behavior (WPF-style code-behind,
    generated `.g.rs` glue, compiled `{x:Bind}` bindings, later an own
    script language) — decided in
    [ADR 017](adr/017-meui-markup-and-code-behind.md). Started: the
    `markup` side is done (line/column spans, the `x:` prefix, dotted
    names and property elements, parsed markup extensions, typed
    `ParseErrorKind`), and the `meridian-ui-mel` generator
    (`.meui` -> `.g.rs`, `meui_include!`; used by `magic_figures`), whose
    `{x:Bind}` compiler parses and validates bindings, resolves them to
    `ui-domain` properties and generates `Names`, `apply_bindings` and
    `write_back_bindings` (type-checked against the `x:ViewModel`).
    A document with a UI root also gets `build_ui`, which creates the
    `UiTree` from the markup, and `.mres` resource sets
    ([meui-resources.md](meui-resources.md)) generate typed constants and
    resolve `@res:Set.name` in attributes; `<Owner.Resources>` blocks and
    `{StaticResource}` resolve at build time, and `LiveUi` patches changed
    attributes of a reparsed document into a live tree, and `HotDocument`
    watches the `.meui`/`.mres` files (debounced, non-blocking) and
    patches or rebuilds the tree (hot reload). Event dispatch is next.

## Scaling beyond a tech demo (Minecraft/Satisfactory-class scope)

Everything above proves the engine works end-to-end for a small scene
(a few dozen renderables, a handful of rigid bodies). A voxel-world game
(Minecraft-like) or a large-factory simulation (Satisfactory-like) needs
real capacity in several places that a small demo never exercises.
Ordered roughly by "blocks everything else" first; items 1 (partly), 2 and 4 are done,
the rest are not started.

1. **Instancing is real; the per-frame instance upload is what is left.**
   `rendering-wgpu` uploads each registered mesh once (`MeshCache`, local
   space, dropped when the mesh is unregistered) and writes one record (120 bytes, ADR 033)
   record per drawn renderable (model matrix, colour, emissive, gloss) into
   a single instance buffer per frame; opaque renderables with equal
   `BatchKey`s (mesh, albedo, lighting, shadow casting) draw in one
   instanced call (`rendering-domain::plan_batches`). Still open for a
   voxel world's or factory's tens of thousands of static instances:
   - A "this instance's record is unchanged, skip the reupload" cache, or a
     persistent instance buffer patched in place, instead of rewriting every
     record each frame.
   - Indirect draws and GPU-side culling (the GPU-driven direction in
     graphics-design.md), and a mesh path for more than 65 535 vertices.
2. **Done: `ecs` multi-component queries.** `World::query_many`
   and `query_many_mut` take tuples of two to four `&T`/`&mut T`
   ("every entity with `Conveyor` *and* `ItemSlot` *and* `Position`").
   Elements can also be `Option<&T>`/`Option<&mut T>` and the
   `With<T>`/`Without<T>` filters, and a `Changed<T>` filter fed by a world
   clock (per-archetype-column granularity). `Commands` record spawn,
   despawn, insert and remove during iteration and apply them afterwards.
   Not built yet: parallel iteration.
3. **`physics-domain::BroadPhase` is x-axis sweep-and-prune** (first
   step, done): `find_candidate_pairs` sorts AABBs by minimum x and only
   tests bodies that overlap along x, returning the same pairs in the same
   order as the exhaustive sweep. Still open for thousands of
   mostly-static colliders (a voxel world's solid blocks, a factory's
   machine footprints) that share an x range: a uniform grid keyed to
   chunk size or a BVH, plus incremental updates for static bodies.
4. **Done: `physics-wgpu`'s batched rigid-body kernels are wired
   into `runtime-app::Runtime`.** (Wave 6, ADR 025: the `PhysicsDispatchMode`
   flag described below became the `PhysicsStepper` port; `BatchedStepper`
   in `physics-wgpu` is the batched mode and `SequentialStepper` in
   `physics-app` the sequential one.) `PhysicsStepStage` (in `runtime-app`,
   re-exported from `meridian-sdk`, alongside its `PhysicsDispatchMode`
   flag) dispatches `RigidBodyIntegratorKernel`/`BroadPhasePairsKernel`/
   `GenerateContactsKernel`/`ConstraintSolverBatchKernel` when in
   `PhysicsDispatchMode::Batched`, instead of `PhysicsSubsystem::step`'s
   plain sequential loop in `PhysicsDispatchMode::Sequential` — proven
   identical bit-for-bit on a single tick and across a 600-tick settling
   regression (`cargo test -p meridian-runtime-app`), and proven live in
   `examples/physic_figures` (swapped in as a drop-in mode change, no
   other example code changed). One type with a dispatch-mode flag, not
   two separate `Stage` types, so an application (or a future third
   mode) switches dispatch shape without touching anything else about
   how it composes its `Runtime`. `runtime-app` moved from tier 7 to
   tier 8 for the new edge (`physics-wgpu` is tier 7 — see
   dependency-rules.md's own note on this edge).

   **Partial progress on the "not literally GPU-executing" gap**: the
   broad-phase overlap test now has a real WGSL kernel
   (`physics-wgpu::float::BroadPhaseGpuKernel`), proven against a
   real GPU device (an actual adapter, not a mock) to produce the exact
   same candidate pairs as `BroadPhase::find_candidate_pairs`. It's
   scoped to just the pairwise AABB comparison — pure arithmetic, no
   geometric-algebra content — because each body's AABB is still
   computed on the CPU via the existing, already-verified
   `RigidBody::aabb()` before upload. The other three pipeline stages
   (the integrator's rotor-exponential/motor-composition math, narrow
   phase's SAT/manifold clipping, the solver's graph-colored Gauss-Seidel
   passes) are genuine GA/geometry code with real porting risk — faithfully
   reproducing them in WGSL (not just "compiles and looks plausible")
   is real, separate follow-up work, not rushed alongside the safe,
   arithmetic-only broad-phase kernel. `PhysicsDispatchMode` currently
   has no `Gpu` variant yet for exactly this reason: adding one before
   all four stages have a real GPU path would silently mix GPU and CPU
   work under one name.
5. **Persistence: started, on Substrate Data.** Every Meridian-owned
   format is [Substrate Data](https://github.com/F000NKKK/Substrate-Data)
   (see [ADR 016](adr/016-data-formats-on-substrate-data.md) and
   [data-formats.md](data-formats.md)) — not `serde` derives scattered
   across every `*-core` type. **Done:** the optional `data` feature on
   `numeric`/`gac` (`Fixed` as raw Q16.16 bits; `Vec3`/
   `Bivector3`/`Rotor`/`Motor3` and their `Fixed*` counterparts as
   `Shape::Named` values, bit-exact through text, binary and frames), and
   the `meridian-data-substrate` crate: an application-built `ComponentRegistry`
   (by name and `TypeId`; `register`/`register_with` a codec —
   `Transform` goes through the `TransformForm` newtype so `ecs`
   stays file-blind), `.mscene` load into a
   `World` in text and binary (`SDAT`, detected by magic; all-or-nothing,
   every problem with path and line/column or byte offset, unknown
   component names with a "did you mean" or kept as written for editors),
   save, `.mprefab` (instancing under `<instance>/<prefab id>`,
   whole-component and field-level overrides (also on generated entities,
   by path), `parent` and references naming generated entities, nesting
   with cycle and depth checks, bytes from a caller's `PrefabSource` port,
   instances saved as the changes from their prefab), the `.msave` envelope (full scene
   document + a game-owned `state` of the game's own schema; the header
   and scene read without decoding the state) and `describe()` for an IDE. **Editor protocol:** the
   `meridian-editor-domain` crate ([editor-domain.md](editor-domain.md))
   defines the typed, append-only `Request`/`Response`/`Event` messages
   between a game and Meridian IDLE (golden-byte pinned, version
   handshake). **Editor host:** `meridian-editor-app` (a pure `Session`
   answering every request against a `World` and a `ComponentRegistry`,
   events by subscription, a `SimulationControl` port) and
   `meridian-editor-tcp` (one editor over loopback TCP, `tokio`); see
   [editor-domain.md](editor-domain.md#host). Parent links load as
   `meridian_ecs::Parent` (`meridian-data-substrate` re-exports it), and
   `propagate_transforms` resolves `Transform`s along it into
   `GlobalTransform`s. `ecs` also has `World::component_types(entity)`
   and in-memory `World::snapshot`/`restore` (memory only: a world's disk
   form is its scene document, see
   [data-formats.md](data-formats.md#how-a-world-is-saved-and-restored)).
   **Not yet done:** a state-level `.msave` delta (a later `format`),
   saving a despawned or reparented generated entity,
   wiring the editor host into `runtime-app`'s frame loop, an importer
   registry and `resource` id table that read `.meta`, and a `.mpak`
   build-tool command (streaming writer). **Asset formats:** `.meta` (stable
   `AssetId`, importer, settings, dependencies) and `.mpak` (checksummed
   payloads + an SDAT index, random access by id or path, versioned
   envelope with a lossless upgrade) are done in `meridian-asset-domain`;
   `asset-app`'s `PackSource` serves a pack as a `ByteSource` and
   `asset-fs`'s `FileReadAt` reads one by range, see
   [data-formats.md](data-formats.md#asset-pack-mpak). **Wired into the sdk:** `meridian_sdk::persistence::WorldFiles` loads and
   saves `.mscene`/`.mprefab`/`.msave` by path, with `AssetPrefabSource`
   implementing `PrefabSource` over `asset-app`'s `AssetLoader` (reads on the
   background pool, ADR 027); a write port (`ByteSink`) is later.
   `substrate-data` is a crates.io dependency (0.3.x).
6. **No networking.** Multiplayer (both games have it) is a subsystem
   that doesn't exist in any form — no netcode, no client/server
   split, no replication. The deterministic `Fixed` numeric mode (see
   [ADR 008](adr/008-fixed-point-determinism.md)) is real groundwork for
   lockstep-style sync, but the actual transport/replication layer is
   unstarted. This is its own multi-month subsystem, not an add-on to
   an existing crate.
7. **UI framework: runtime, fill and text output started.**
   `meridian-ui-domain` ([ADR 018](adr/018-ui-architecture.md)) has the
   WPF-style element tree, typed properties, measure/arrange layout
   (`Border`, `StackPanel`, `DockPanel`, `Grid`, `TextBlock`) and hit
   testing, focus and input routing (`handle_input`, `UiEvent`s, a
   consumed/ignored outcome), tested without a GPU.
   `meridian-ui-scene2d` extracts `Background` fills and `TextBlock` glyphs to `Scene2d`
   ([ADR 021](adr/021-ui-extraction-and-extensibility.md)); there is no
   sprite renderer for `Scene2d` yet. `meridian-text-domain`
   ([ADR 023](adr/023-own-font-stack.md)) parses TrueType, rasterizes and
   atlases glyphs. Still missing: controls, styles and templates. Markup literals (`"Auto"`, `"2*"`, `"4,2"`) parse through
   `PropertyType::parse`.
8. **Asset pipeline stops at static meshes.** `asset-domain::ObjDecoder`
   has no skeletal/skinning support and there's no animation system at
   all — a blocker for any animated creature (Minecraft mobs) or
   animated machine (Satisfactory conveyors, at least beyond a
   procedural spin). glTF (which carries skinning data OBJ can't) is
   the natural next format, not a bigger OBJ. Static glTF models are done
   ([ADR 030](adr/030-gltf-static-models-through-an-asset-adapter.md));
   skinning and animation stay open.

**Rough sequencing, if picking one order**: (4) (batched physics
dispatch) is done. (1) and (2) next — they're the two things that make
*any* further content-heavy work (chunks/factories, gameplay systems)
viable at all, not nice-to-haves. (3) (spatial broad-phase) after that,
once there's actually a large scene to profile against. (5)
(persistence — the format and scene load/save exist now, see above)
before (6) (networking) — replication design usually wants a
serialization format to already exist. (7)/(8) (UI, animation)
can start in parallel with the above once a concrete game loop exists
to build them against — see "Why implementation is deliberately last"
below for why this workspace prefers building against a real,
exercised need over speculative scaffolding.

This is a realistic multi-month-per-item undertaking each, not a single
sprint — treat this section as "what's missing," not "next release."

## Explicitly out of scope for now

- `animation-core`, `particles-core`, `ai-core` — referenced in
  [dependency-rules.md](dependency-rules.md) as future consumers of
  `compute-app`, but not part of the current workspace. Add them only when
  there's a concrete subsystem to build, not speculatively.
- Splitting `rendering-wgpu` into a separately-named RHI crate plus backend
  crates (`vulkan-driver`, etc.) — `rendering-wgpu` already plays the RHI
  role today (see [graphics-design.md](graphics-design.md)); a rename or
  further split is only worth doing once a second concrete backend actually
  exists to justify it.

## Not yet decided

- **Deterministic simulation mode — decided: Q16.16 fixed-point
  (`meridian-numeric::Fixed`), not ordered floating point.** Ordered
  floating point (careful summation order, no FMA, disabled
  auto-vectorization) was rejected: it fights the compiler/CPU rather
  than sidestepping the problem, and still doesn't fully solve
  cross-`libm` `sin`/`cos`/`sqrt` disagreement, which fixed-point +
  CORDIC avoids by construction (integer add/subtract/shift only). See
  "Current state" above for what's built (`gac::fixed_ga`,
  `physics-domain::fixed`) and [ADR
  008](adr/008-fixed-point-determinism.md).
- **GPU backend beneath `compute-wgpu`/`rendering-wgpu`/`physics-driver`
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
  `naga` + platform GPU bindings), and `async`-flavored device/adapter
  acquisition — see the `tokio` entry below for how that's bridged.
  `rendering-wgpu`'s existing stub shape (`Device`, `CommandBuffer`,
  `Buffer`, `Texture`, `Shader`, `Pipeline`) already maps onto `wgpu`'s
  own vocabulary, so this doesn't force a redesign, just a real
  implementation of what's already stubbed. `platform-winit`'s
  `Window`/`DynamicLibrary` are a separate, smaller decision (below) and
  keep their own hand-written-FFI answer; GPU is the one deliberate
  exception to zero-deps, not a reversal of the policy in general.

  **`rendering-wgpu` and `compute-wgpu` are both real now**, sharing
  their `wgpu` device/buffer/shader/compute-pipeline mechanics through a
  third crate, `meridian-gpu-wgpu` — see
  [ADR 011](adr/011-shared-gpu-driver-crate.md). `rendering-wgpu::Device`
  supports both headless (`Device::new`) and windowed
  (`Device::new_windowed`, a real swapchain `Surface`) construction, with
  a real render pipeline (`RenderPipeline`, `DepthTexture`,
  `VertexLayout`) — proven end-to-end by the `magic_figures`/
  `physic_figures` examples (real windowed scenes, not headless). `compute-wgpu` gained
  `GpuComputeDevice`, a second backend alongside its existing CPU
  `ComputeDevice`, and `compute-app::ComputeContext::with_gpu` is the
  real dispatch path domain crates reach it through (per rule 5) — proven
  by a compute-shader round-trip test reachable through
  `compute-app` itself, not just `compute-wgpu` in isolation.
  `platform-winit::GpuCapabilities` gained its first real field
  (`device_name`), populated by both drivers' `BackendCapabilities` impls
  (via `meridian_gpu_wgpu::Device`'s own impl); `physics-driver`/
  `audio-cpal` still report `gpu: None` since neither dispatches to a
  GPU yet.
- **Async I/O — decided: `tokio`, scoped to genuine I/O only, not applied
  uniformly** (asset file reads are the exception: background threads,
  [ADR 027](adr/027-background-asset-loading.md)). `rendering-wgpu::Device::new`/`read_buffer` (an OS/driver
  handshake and waiting on in-flight GPU work — both genuinely unbounded
  wait, no useful CPU work to do meanwhile) are real `async fn`s now, not
  `pollster`-wrapped synchronous functions; `read_buffer`'s manual
  `wgpu::Device::poll` (needed since `wgpu` has no reactor integration of
  its own) runs inside `tokio::task::spawn_blocking` so it can't stall
  other work sharing the runtime. Recording/allocation calls
  (`create_buffer`, `write_buffer`, `CommandBuffer::submit`, ...) stay
  synchronous — bounded, local, effectively-instant work gains nothing
  from being `async`, the same reason `Vec::push` isn't. `tokio` is only
  a dependency of crates with a genuine I/O operation of their own —
  `audio-cpal`'s real `AudioDevice::new` is an `async fn` per the same
  policy (and needs `tokio` itself only as a dev-dependency, for its
  tests' runtime); `platform-winit::Window`/`DynamicLibrary` will add it
  if their implementations gain a genuine I/O operation. It isn't forced
  onto `ecs`,
  `gac`, `physics-domain`, or anything else with no I/O. See
  [ADR 009](adr/009-async-io-via-tokio.md) for the full decision,
  including why this reverses this document's own earlier "not a reason
  to pull in a full async runtime for one call" note (that reasoning held
  only while GPU acquisition was the workspace's *only* I/O-shaped
  operation).
- **`meridian-audio-effects` (heavier DSP effects as a separate crate)** —
  decided: not yet, and only split it out when a concrete effect actually
  needs an external dependency. `meridian-audio-domain` already owns basic,
  zero-dependency DSP (`DspNode`/`DspGraph`, `Gain`, `LowPassFilter`) and
  should keep owning simple effects that need nothing beyond
  `numeric`/`gac` — there's no architectural reason to move those
  out. The trigger for a separate crate is the same class of decision as
  `wgpu` below: something like a real convolution reverb, multiband EQ, or
  resampling needs an FFT crate (e.g. `rustfft`), and pulling that into
  `audio-domain` would force every consumer of basic spatial audio (including
  `runtime-app` and every example) to compile it too. When that concrete
  need exists, `meridian-audio-effects` depends on `meridian-audio-domain`
  only (implements its `DspNode` trait, one edge, no new adapter pattern —
  unlike `gac-wgpu` this isn't avoiding a forbidden edge, just isolating
  an optional heavy dependency) and ships the heavy effects there, keeping
  `audio-domain`'s own dependency footprint minimal. Creating the crate before
  that concrete need exists would be exactly the speculative split
  `roadmap.md` already rejects elsewhere (see "Explicitly out of scope").
- **`meridian-platform-winit`'s `Window` — decided: `winit`, not
  hand-written per-platform FFI.** Reversed from the original plan (see
  [ADR 010](adr/010-windowing-via-winit.md) for the full decision):
  correct cross-platform windowing (lifecycle, input delivery, HiDPI,
  IME) turned out to be the same class of multi-month,
  multiple-independent-bug-classes undertaking that justified accepting
  `wgpu` over hand-written Vulkan/DX12/Metal — the "small enough to
  hand-roll safely" reasoning didn't survive contact with the real scope.
  `platform-winit::Window` wraps a real `Arc<winit::window::Window>`;
  `meridian-rendering-wgpu` stays `winit`-agnostic (`Device::new_windowed`
  takes `impl Into<wgpu::SurfaceTarget<'static>>`, a `wgpu`-defined bound,
  not a `winit`-specific type). `platform-winit`'s existing
  `KeyCode`/`MouseButton`/`InputState` vocabulary needed no redesign —
  real `winit` event handling translates into that existing API.
  `DynamicLibrary` is unaffected by this decision: still hand-written FFI
  (`dlopen`/`LoadLibrary`) — genuinely small, no ecosystem-standard crate
  the way `winit` is for windowing.
