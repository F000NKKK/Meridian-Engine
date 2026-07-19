# ADR 008: Fixed-point determinism via a disclosed float_ga/fixed_ga split

## Status

Accepted

## Context

Reproducible replay and lockstep networked simulation both need the same
sequence of operations on the same inputs to produce bit-identical results
across every machine that runs them. `f32`/`f64` cannot promise that:
IEEE-754 leaves rounding-mode, FMA fusion, and extended-precision-register
behavior implementation-defined, and different compilers/CPUs/optimization
levels really do disagree on the last bit or two of `+`/`*`/`sin`/`sqrt` in
practice. In a lockstep scheme where only player inputs travel over the
network and every client simulates locally, that tiny per-frame divergence
compounds (physics is sensitive to initial conditions) into full desync
within seconds.

`gac-core::Motor3` (and everything built on it — `Vec3`, `Rotor`,
`Bivector3`) is hardcoded to `Scalar = f32`. Two ways to get a
deterministic alternative: make `gac-core` generic over the scalar type,
or add a second, `Fixed`-point-based implementation alongside the existing
one.

## Decision

**A fixed-point type, not "careful" floating point.** `Fixed`
(`meridian-numeric-core::Fixed`) is Q16.16 (`i32`-backed): 16 integer bits
(range ±32768), 16 fractional bits (resolution ~1.5e-5). `+`/`-`/`*`/`/` on
`i32`/`i64` are exactly specified by the language, so `Fixed` is exactly
reproducible by construction — no rounding-mode/FMA/vectorization
variance to fight. `sqrt` is Newton's method on the raw integer (exact,
not a polynomial fit). `sin`/`cos`/`atan2` use
[CORDIC](https://en.wikipedia.org/wiki/CORDIC): the standard way to
compute trig from only add/subtract/shift, which is what keeps them
reproducible too — a polynomial approximation or lookup table backed by a
`libm` call would reintroduce exactly the platform dependence `Fixed`
exists to remove. "Ordered floating point" (careful summation order, no
FMA, disabled auto-vectorization) was considered and rejected: it fights
the compiler/CPU rather than sidestepping the problem, and still doesn't
fully solve cross-`libm` `sin`/`cos` disagreement.

**Two GA modules in `gac-core`, not one generic implementation.**
`gac-core::float_ga` (the existing `f32` `Multivector`/`Vec3`/`Bivector3`/
`Rotor`/`Motor3`, re-exported at the crate root so nothing downstream
changed) and `gac-core::fixed_ga` (a disclosed, one-for-one duplication of
the same structure, built on `Fixed`). Making the existing types generic
over the scalar type instead was considered and rejected specifically
because of `gac-core`'s own compute-batching story
([ADR 007](007-batch-transforms-via-compute.md)): `meridian-gac-compute`
dispatches batched `Motor3` operations to GPU via `compute-runtime`, and
GPUs have no good answer for fixed-point at all — they're `f32`-native
hardware with no real `i64` support (`Fixed`'s multiply/divide need an
`i64` intermediate to avoid overflow), so a GPU-dispatchable `Motor3` has
to stay floating-point regardless of how the CPU-side type is
parameterized. GPU execution also has its own source of nondeterminism
(warp/wavefront scheduling order affecting reduction results) independent
of the arithmetic type, so a generic `Motor3<S>` wouldn't even deliver
GPU-side determinism if it did compile there. A `Fixed` instantiation of a
generic `Motor3<S>` would still need this exact duplication of blade
multiplication logic at the point of instantiation to keep the GPU path
pure `f32`, so genericity buys nothing here beyond what a second concrete
module gives directly, at the cost of a shared generic engine that every
call site (including the GPU-relevant one) has to reason about.

**Opt-in, not a mode switch.** `physics-core::deterministic` mirrors the
default `f32` pipeline one-for-one — `DeterministicBody` ~ `RigidBody`,
`DeterministicIntegrator` ~ `Integrator`, `DeterministicConstraintSolver`
~ `ConstraintSolver`, `DeterministicBroadPhase`/`DeterministicNarrowPhase`
~ `BroadPhase`/`NarrowPhase` — as a genuinely separate, parallel path.
`RigidBody` and the rest of `physics-core`'s default pipeline are
untouched; nothing in the engine has to know a simulation is running
deterministically unless it explicitly constructs `DeterministicBody`s.
`DeterministicBody::frame_f32` converts the pose to `gac-core::Motor3` for
handoff to rendering/ECS/audio, which stay in `f32` regardless of which
physics path produced the pose — a direct component-wise conversion of
the underlying 16-component multivector (both types expose theirs, `pub`),
not a re-derivation through rotation/translation.

**`fixed_ga` owns geometric primitives too, not just algebra — and not
`physics-core`.** `FixedAabb`/`FixedSphere`/`FixedObb`/`FixedCone`/
`FixedPlane`/`FixedShape`/`FixedConvexVolume` mirror `float_ga`'s
primitive set one-for-one, living in `gac-core`, the same place
`float_ga`'s primitives do. An early version of this work put a
sphere-only `DeterministicShape` and hand-rolled broad-phase overlap math
directly inside `physics-core::deterministic` — caught on review as a
layering violation: geometric primitives are `gac-core`'s
responsibility regardless of scalar flavor (the same reason `Aabb` was
moved out of `physics-core` into `gac-core` earlier in this project's
history, once it turned out `physics-core` and `graphics-core` had each
written the same one independently). Locking `Fixed` primitives inside
`physics-core` would have blocked exactly the kind of reuse `gac-core`
exists to enable — a deterministic `graphics-core` CPU path, or a large
precise CPU/GPU-emulated simulation, would have had no way to reach them
without depending on physics. `physics-core::deterministic` now builds
its `FixedAabb`s from `gac-core::fixed_ga` the same way its `f32`
`BroadPhase` builds `Aabb`s from `gac-core::float_ga`.

**Cross-flavor interop is named, not `From`/`Into`.** Both flavors
already share the same method names by construction (`dot`, `compose`,
`transform_point`, ...), so converting between them is `to_fixed_lossy`/
`to_float_lossy` — explicit, precision-changing casts named as such at
every call site, plus mixed-type `Add`/`Sub`/`Mul` (`Vec3 + FixedVec3`,
...) built on those same named conversions internally. `From`/`Into` was
considered and rejected: `.into()` makes a cast that changes both
precision *and* the determinism guarantee look free, which invites
exactly the accidental use this ADR's "ordered floating point" rejection
above is also about — a silent conversion nobody notices at review time.
A custom `#[deprecated]`-style compiler warning was also considered (repurposing
that lint to force `#[allow(deprecated)]` at each call site) and rejected
as a misuse of an attribute whose stated meaning is "will be removed", not
"deliberate lossy cast" — a self-describing method name achieves the same
call-site visibility without repurposing unrelated compiler machinery.

**`gac-compute` gets `Fixed` kernels too, CPU-dispatch only.**
`FixedMotorTransformKernel`/`FixedMotorComposeKernel` mirror
`MotorTransformKernel`/`MotorComposeKernel`, built on
`fixed_ga::FixedMotor3`. Restricted to CPU dispatch by convention/doc
comment, not by a type-level distinction — `compute-runtime` has no GPU
backend implemented yet (see [roadmap.md](../roadmap.md)), so there's
nothing to restrict against today; the restriction is recorded now so a
future GPU backend doesn't accidentally get offered `Fixed` kernels, for
the same GPU-can't-do-fixed-point-or-determinism reasons as everywhere
else in this ADR.

**Sphere colliders only, for now, in `physics-core::deterministic`
specifically.** `Cuboid`/SAT collision *response* (contact
generation with point/normal/depth — a different, harder problem than
the containment test `Shape`/`ConvexVolume` answer, see
docs/physics-design.md) was not ported to the deterministic pipeline in
this pass. That's a large, intricate piece of code, and porting it
hastily risks subtle bugs in exactly the code whose entire purpose is
trustworthy reproducibility. Tracked as explicit follow-up work in
[roadmap.md](../roadmap.md), not silently dropped. `fixed_ga::FixedObb`
itself already exists (see above), so this is narrower than it was
before: only the SAT contact-generation algorithm remains to be ported,
not the underlying primitive.

## Alternatives considered

- **Ordered/careful floating point** (fixed summation order, `-ffp-contract=off`
  equivalent, disabled auto-vectorization) — rejected: doesn't solve
  cross-`libm` transcendental disagreement, and constrains the compiler/CPU
  in ways that are easy to violate accidentally in future code (a single
  auto-vectorized loop or FMA-fused multiply-add silently reintroduces the
  exact problem this exists to solve, with no compiler error to catch it).
- **Generic `gac-core` types parameterized over the scalar** — rejected:
  see "Decision" above — `gac-compute`'s GPU dispatch path has no good
  answer for `Fixed` regardless of genericity, so the duplication this
  would avoid still has to exist at the GPU-relevant instantiation site.
- **A software/soft-float deterministic float type** (e.g. a portable
  IEEE-754 emulation layer) instead of fixed-point — not pursued: still
  needs a from-scratch deterministic `sin`/`cos`/`sqrt` implementation
  (the actual hard part `Fixed`+CORDIC already solves), for a wider
  dynamic range this workspace's physics-scale quantities don't need, at
  higher CPU cost than native integer arithmetic.

## Consequences

- `Fixed` and `gac-core::fixed_ga` (algebra *and* primitives) are real,
  independently tested (cross-checked against `f64`/`float_ga` oracles),
  and proven with an actual bit-exact reproducibility test — not just
  "close" — in `physics-core::deterministic`.
- `float_ga` (and everything built on `f32`, including the GPU compute
  path) is completely unaffected — no downstream crate changed to make
  this possible.
- `fixed_ga`'s primitives are reusable by any future crate that needs
  CPU-deterministic geometry, not gated behind `physics-core` — the
  specific property the "not `physics-core`" correction above exists to
  preserve.
- `gac-compute` carries `Fixed` kernels alongside its `f32` ones now, so
  batch-transform work has a deterministic path too, once something
  needs it at scale (today: nothing does yet, `physics-core::deterministic`
  calls `fixed_ga` directly per-body).
- A second collider shape's *collision response* (SAT contact generation
  for `Cuboid`, specifically) or a second compute domain choosing
  fixed-point would each need their own disclosed duplication into
  `fixed_ga`-adjacent code, following the same pattern — accepted as the
  cost of keeping the GPU-dispatchable path pure `f32` without a generic
  scalar abstraction leaking into it.
- Determinism is a CPU-only guarantee in this workspace: nothing here
  makes GPU-dispatched work (rendering, GPU-side batch transforms)
  reproducible, and nothing tries to — that's a structurally different
  problem (see "Decision" above).
