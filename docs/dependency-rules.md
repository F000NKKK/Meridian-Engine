# Dependency rules

The workspace graph is a DAG on purpose. This document is the ruling
reference when a PR (human or agent-authored) is unsure which direction a
`use` or a `Cargo.toml` dependency is allowed to point.

## Direction

Dependencies point inwards ([ADR 019](adr/019-ports-in-domain-drivers-as-adapters.md),
Craftbook ch. 7 §7.3): interface → application → domain ← infrastructure,
and every layer may use the shared kernel. Domain and application crates own
the port traits they need; driver and `*-compute` crates are adapters that
depend on those crates and implement the ports. A domain or application
crate never depends on a driver.

Each crate's layer and bounded context is declared once, in
[`craftbook.toml`](../craftbook.toml) (reading aid:
[context-map.md](context-map.md)). `./build.sh check-deps` rejects any edge
that points outwards unless it is listed in `PENDING` in
`scripts/check_dependency_rules.py`, marked *migration pending* with the wave
that inverts it. Those edges are the ADR 005 graph not yet migrated; the list
only shrinks, and no new edge may be added to it. They are marked
**(pending, wave N)** in ["All edges, exactly"](#all-edges-exactly).

## The graph

This section is a *reading aid*, not the authoritative rule — the exact,
enforced graph is `ALLOWED` in `scripts/check_dependency_rules.py`
(`./build.sh check-deps` fails the build the moment a `Cargo.toml` edge
and that dict disagree, in either direction). The diagram below is kept
in sync with it, but if the two ever conflict, `ALLOWED` wins; fix the
diagram, not the other way around. The full, exact per-crate edge list
is in ["All edges, exactly"](#all-edges-exactly) below — read the
diagram for the shape of the graph, that section for the ground truth
of any one crate's dependencies.

`meridian-foundation`, `meridian-memory-core`, and `meridian-task-core`
are the graph's independent leaves — each depends on nothing (rule 0
covers `foundation` specifically: *any* other crate may take an edge to
it, so it isn't drawn as a dependency of anything below).

The real graph has too many crisscrossing edges (many crates each
depend directly on `foundation`, `gac-core`, `resource-core`, ... —
see ["All edges, exactly"](#all-edges-exactly)) to draw as a single
arrow-accurate ASCII diagram without either lying by omission or
becoming unreadable clutter — the previous version of this section
tried the "one diagram, arrows glossed over with a disclaimer"
approach and it drifted from the real graph more than once. Instead,
here's every crate grouped by dependency depth (0 = a leaf; N = one
more than the deepest thing it depends on) — a crate in tier N can only
depend on crates in tiers `0..N`, never a same-tier or higher-tier one
(that's what makes the graph a DAG):

```text
tier 0  foundation, memory-core, task-core, dsl-macros
tier 1  asset-core, dsl-core, numeric-core, platform-core, resource-core, ui-core
tier 2  audio-driver, gac-core, gpu-driver, mel-build
tier 3  audio-core, compute-driver, ecs-core, graphics-driver, physics-core
tier 4  compute-runtime, graphics-core, scene
tier 5  editor-protocol, gac-compute
tier 6  editor-host, physics-compute
tier 7  editor-tcp, engine-core
tier 8  sdk
```

`meridian-dsl-core`/`meridian-dsl-macros` are the two crates behind the
SDK's extensible scene DSL (see [ADR 015](adr/015-extensible-scene-dsl.md)
and `meridian-sdk::dsl`'s own module doc): `dsl-core` is a domain-blind
tag/attribute-markup parser plus the generic `DslTag`/`DslRegistry`
machinery (tier 1: it depends only on `foundation`, for its
`EngineError` impls); `dsl-macros` is the `#[dsl_tag(name = "...")]`
proc-macro that implements `DslTag` for a plain struct (tier 0: a
proc-macro crate, no meridian-* runtime dependency at all — it depends
on `syn`/`quote`/`proc-macro-crate` externally, not shown here since
this diagram only tracks internal `meridian-*` edges). Neither knows
about `Entity`/`Mesh`/`RigidBody` or any other specific tag — those are
defined in `meridian-sdk::dsl` (built-ins) or in a game's own crate
(custom tags), both just ordinary `#[dsl_tag]`-annotated structs.
`meridian-mel-build` (tier 2, on `dsl-core` and `ui-core`; the latter only
to resolve a bound attribute to its UI property at build time) is the build-time
`.mel` -> `.g.rs` generator of [ADR 017](adr/017-mel-markup-and-code-behind.md).
An application takes it as a **build-dependency** for its `build.rs`,
the one exception to "applications depend on `meridian-sdk` alone": a
build script cannot use the SDK without compiling the whole engine for
the host, and the generated code itself names only `meridian_sdk`.

`meridian-scene` is scene serialization on Substrate Data (see
[ADR 016](adr/016-data-formats-on-substrate-data.md) and
[data-formats.md](data-formats.md)): an application-built
`ComponentRegistry` and `.mscene` load/save into an `ecs-core::World`. It
depends on `ecs-core` (the `World` it fills) and on `gac-core` with its
optional `data` feature (`Motor3`'s schema, for `Transform`), plus the
external `substrate-data`. It sits *above* `ecs-core` on purpose, so
`ecs-core` stays file-blind (rule 3): `Transform` is stored through a
`TransformForm` newtype in `meridian-scene` rather than `ecs-core` growing a
format dependency, and the scene's `Parent` component lives there too
until `ecs-core` has a hierarchy of its own. Nothing depends on it yet; `engine-core`/`sdk` will when the
editor host and scene loading are wired up (docs/roadmap.md). The
`data` features of `numeric-core`/`gac-core` add no internal edge — only
the optional external `substrate-data` dependency.

`meridian-editor-protocol` is the wire protocol between a running game and
its editor (Meridian IDLE): typed `Request`/`Response`/`Event` messages as
Substrate Data frames (see [editor-protocol.md](editor-protocol.md)). It
sits *above* `meridian-scene` because it reuses its `ComponentName` and
carries `ComponentRegistry::describe()` unchanged, and takes `ecs-core` only
for `Entity`'s shape (`EntityRef`). Pure protocol: no `async`, no sockets,
no I/O beyond a generic `Read`/`Write`.

`meridian-editor-host` (tier 6) is the game's side of that protocol without
I/O: a `Session` that answers decoded requests against a `World` and a
`ComponentRegistry`, a `SimulationControl` port the frame loop implements,
and a `RequestHandler` port a transport calls. It sits above
`editor-protocol`, `scene` and `ecs-core` and needs no edge beyond those;
it is *infra* in `craftbook.toml` because the protocol crate it speaks is
(an app crate may not depend on infra, ADR 019). `meridian-editor-tcp`
(tier 7) is its adapter: the only crate of the editor host with `tokio`,
serving one editor over loopback TCP with `substrate_data::io` frames.
Neither depends on `engine-core`, and `engine-core` does not depend on
them yet: the wiring is a follow-up ([editor-protocol.md](editor-protocol.md#host)).

`meridian-ui-core` (tier 1, on `foundation` and `memory-core` only) is the
WPF-style UI runtime of [ADR 018](adr/018-ui-architecture.md): element
tree, typed properties, layout and hit testing. It deliberately has no
edge to `graphics-core`, `platform-core` or `ecs-core`: turning a laid-out
tree into `Scene2D` renderables and feeding it `InputState` are the job of
`meridian-ui-render` (the adapter above both `ui-core` and `graphics-core`,
ADR 021) does the first half; the second is a later adapter above
`platform-core`.

This shows *depth*, not *which crate depends on which* — two crates in
the same tier aren't necessarily related at all (`audio-core` and
`ecs-core` share tier 3 but neither depends on the other). For the
actual edges, use ["All edges, exactly"](#all-edges-exactly); don't
infer them from tier adjacency.

`meridian-graphics-driver` and `meridian-compute-driver` both also depend
on `meridian-gpu-driver` (omitted from the diagram above for
readability, the same way the `gac-compute` edges below are) — the crate
that owns the actual `wgpu` device/buffer/shader/compute-pipeline
mechanics shared between rendering and general GPU compute, so neither
driver reimplements it independently or reaches into the other's crate
to get at it:

```text
        meridian-platform-core
                   |
          meridian-gpu-driver
                   |
        +----------+----------+
        |                     |
graphics-driver          compute-driver
```

See [ADR 011](adr/011-shared-gpu-driver-crate.md) for the full decision.

`meridian-gac-core` and `meridian-compute-runtime` never depend on each
other — geometric algebra ("what to compute") and the compute dispatch
runtime ("where to compute it") are deliberately independent. The adapter
between them is its own crate:

```text
meridian-gac-core   meridian-compute-runtime
        |                     |
        +----------+----------+
                   |
         meridian-gac-compute
                    |
        +-----------+-----------+
        |                       |
  graphics-core            physics-core
```

`meridian-gac-compute` depends on both and implements `compute-runtime`'s
`ComputeKernel` trait for GAC batch operations (`MotorTransformKernel`,
`MotorComposeKernel`, ...). `graphics-core` depends on
`gac-compute` for batched transform work and on `compute-runtime` directly
for non-GAC compute (e.g. GPU culling) — both edges omitted from the main
diagram above for readability. `physics-core` has neither edge: its batching
lives in `physics-compute`. Also omitted: `meridian-audio-core`'s
dependency on `meridian-gac-core`/
`meridian-resource-core` and `meridian-graphics-core`'s/
`meridian-physics-core`'s own direct dependency on `meridian-resource-core`
(handle types their `MeshRegistry`/`ColliderMeshHandle` etc. use) — see
["All edges, exactly"](#all-edges-exactly) for the complete, non-abridged
list.

`meridian-gac-compute` and `meridian-physics-compute` are adapters of the
compute context: they depend on `meridian-compute-driver` (the adapter that
implements `compute-runtime`'s `ComputeDevice` and `GpuBackend` ports) and on
`meridian-gpu-driver`. A kernel still dispatches through
`compute-runtime::ComputeContext`, never around it; it reaches the GPU with
`ComputeContext::gpu::<compute_driver::GpuComputeDevice>()`, whose
buffer/shader/pipeline methods
are typed in terms of `gpu-driver`'s own resource types
(`Buffer`/`BufferUsage`/`Shader`/`ComputePipeline`) — a real GPU-dispatching
`ComputeKernel` has to name those types (a bind-group's usage flags, a
kernel's own pipeline field, ...), which needs `gpu-driver` in scope. This
doesn't reach around `compute-runtime`'s dispatch mechanism (rule 5's
actual concern); it's the same category of edge `graphics-driver`/
`compute-driver` already have to `gpu-driver` for the same
resource-type-naming reason — see
[ADR 011](adr/011-shared-gpu-driver-crate.md). See
[ADR 007](adr/007-batch-transforms-via-compute.md).

`meridian-physics-compute` is rule 11's first concrete
`meridian-<domain>-compute` adapter: `physics-core`'s deterministic
soft-body GPU kernels (mass-spring integration), mirroring
`gac-compute`'s shape one layer up:

```text
meridian-physics-core   meridian-compute-runtime
        |                        |
        +-----------+------------+
                     |
          meridian-physics-compute
```

Same reasoning as `gac-compute` throughout: `physics-compute` depends
directly on `meridian-gpu-driver` for the same resource-type-naming
reason, and also on `meridian-gac-core` (the `GaFlavor`/`Plane`/`Vec3`
types its kernels' signatures are expressed in) and `meridian-gac-compute`
(its `Fixed`-flavor kernel reuses `fixed_wgsl::FIXED_ARITHMETIC_LIB_WGSL`
rather than re-deriving the same Q16.16 emulation — see
`meridian-physics-compute`'s own module doc). It also takes rule 0's
open `meridian-foundation` edge: `narrow_phase::GenerateContactsKernel`
logs via `log_warn!` when a pair's manifold exceeds its fixed
`MAX_CONTACTS_PER_PAIR` output slot, the same `MAX_LIGHTS`-style
truncation policy `graphics-core::submission` uses.

`meridian-engine-core` depends on `meridian-physics-compute` too (moving
`engine-core` from tier 7 to tier 8, since it now sits above
`physics-compute` rather than beside it) — `Runtime`'s
`PhysicsComputeStepStage` (see that crate's own module doc) dispatches
`physics-compute`'s batched `RigidBodyIntegratorKernel`/
`BroadPhasePairsKernel`/`GenerateContactsKernel`/
`ConstraintSolverBatchKernel` instead of `PhysicsSubsystem::step`'s
plain sequential loop — the concrete answer to "physics needs to scale
past a handful of bodies" docs/roadmap.md's "Scaling beyond a tech
demo" section names. Not a new coordination-logic violation of rule 7:
`engine-core` is still the *only* crate reaching into `physics-compute`
for this, and `PhysicsComputeStepStage` is a thin `Stage` wrapper
dispatching real batched kernels, not a reimplementation (CLAUDE.md's
"don't drag another crate's logic into your own" rule).

## All edges, exactly

The ground truth: every crate's real `[dependencies]`, mirroring
`ALLOWED` in `scripts/check_dependency_rules.py` (`./build.sh
check-deps` fails if a real edge and this list disagree — regenerate
both together when either changes, in the same PR). Ordered roughly
bottom-up.

- `meridian-foundation` -> (nothing)
- `meridian-memory-core` -> (nothing)
- `meridian-task-core` -> (nothing)
- `meridian-numeric-core` -> `meridian-foundation`
- `meridian-platform-core` -> `meridian-foundation`
- `meridian-gac-core` -> `meridian-numeric-core`
- `meridian-resource-core` -> `meridian-memory-core`
- `meridian-ecs-core` -> `meridian-gac-core`, `meridian-memory-core`
- `meridian-gpu-driver` -> `meridian-foundation`, `meridian-platform-core`
- `meridian-graphics-driver` -> `meridian-foundation`, `meridian-gpu-driver`, `meridian-platform-core`
- `meridian-compute-driver` -> `meridian-compute-runtime`, `meridian-gpu-driver`, `meridian-platform-core`
- `meridian-audio-driver` -> `meridian-audio-core`, `meridian-foundation`, `meridian-platform-core`
- `meridian-compute-runtime` -> (none; it owns the `ComputeDevice` and `GpuBackend` ports)
- `meridian-gac-compute` -> `meridian-compute-driver`, `meridian-compute-runtime`, `meridian-gac-core`, `meridian-gpu-driver`, `meridian-numeric-core`
- `meridian-asset-core` -> `meridian-foundation`
- `meridian-graphics-core` -> `meridian-asset-core`, `meridian-ecs-core`, `meridian-foundation`, `meridian-gac-core`, `meridian-graphics-driver` (pending, wave 5), `meridian-memory-core`, `meridian-resource-core`
- `meridian-audio-core` -> `meridian-gac-core`
- `meridian-physics-core` -> `meridian-gac-core`, `meridian-numeric-core`, `meridian-resource-core`
- `meridian-physics-compute` -> `meridian-compute-driver`, `meridian-compute-runtime`, `meridian-foundation`, `meridian-gac-compute`, `meridian-gac-core`, `meridian-gpu-driver`, `meridian-numeric-core`, `meridian-physics-core`
- `meridian-engine-core` -> `meridian-audio-core`, `meridian-compute-runtime`, `meridian-ecs-core`, `meridian-foundation`, `meridian-gac-core`, `meridian-physics-compute` (pending, wave 6), `meridian-physics-core`, `meridian-platform-core` (pending, wave 6), `meridian-task-core` (pending, wave 6)
- `meridian-scene` -> `meridian-ecs-core`, `meridian-gac-core`
- `meridian-editor-protocol` -> `meridian-ecs-core`, `meridian-scene`
- `meridian-editor-host` -> `meridian-ecs-core`, `meridian-editor-protocol`, `meridian-scene`
- `meridian-editor-tcp` -> `meridian-editor-host`, `meridian-editor-protocol`
- `meridian-dsl-core` -> `meridian-foundation`
- `meridian-dsl-macros` -> (nothing; external `syn`/`quote`/`proc-macro-crate` only)
- `meridian-mel-build` -> `meridian-dsl-core`, `meridian-ui-core`
- `meridian-ui-core` -> `meridian-foundation`, `meridian-memory-core`
- `meridian-ui-render` -> `meridian-gac-core`, `meridian-graphics-core`, `meridian-ui-core`
- `meridian-sdk` -> `meridian-asset-core`, `meridian-audio-core`, `meridian-audio-driver`, `meridian-compute-driver`, `meridian-compute-runtime`, `meridian-dsl-core`, `meridian-dsl-macros`, `meridian-engine-core`, `meridian-foundation`, `meridian-gac-core`, `meridian-graphics-core`, `meridian-graphics-driver`, `meridian-physics-core`, `meridian-platform-core`, `meridian-ui-core`

`meridian-engine-core` has no edge to `meridian-graphics-core` or
`meridian-asset-core`: `Runtime::tick` doesn't render a frame or load an
asset (see that crate's own module doc and docs/roadmap.md's
`graphics-core`/`Runtime::tick` entry). Rule 7 would allow the edges the day
a frame step needs them; add them then, not before.

**`meridian-sdk` is the composition root** (interface layer,
[ADR 020](adr/020-craftbook-adoption.md) D3): the single application-facing
re-export surface and the one library crate that names concrete adapters
(`graphics-driver`, here, for windowed rendering). The interface layer may
depend on every layer; what it may not hold is business logic. Its
`scene`/`camera`/`render`/`dsl` logic is scheduled to move to the context
crates it belongs to (wave 6). Applications depend on `sdk` alone.

## Rules

0. **`meridian-foundation` is the open bottom of the graph.** It depends
   on nothing (its optional `file-logging` feature is the one external
   exception, pulling `tokio` for the buffered async log-file sink), and
   *any* crate may take an edge to it — that edge is added to
   `scripts/check_dependency_rules.py` when actually taken, not
   pre-declared. It exists for exactly two kinds of content: shared
   conventions (`EngineError`, `FeatureFlags`) and process-wide
   diagnostics (the unified `logging` sink and `crash_reporting` panic
   hook). Diagnostics are deliberately *not* a "global manager" in rule
   3's sense: the sink owns no engine objects and hands out no handles —
   it appends lines, the same category as `std`'s own panic hook. It is
   the one global exempted in `craftbook.toml`; its file sink and crash
   report are I/O and move to an adapter crate, with the sink installed by
   the composition root ([ADR 020](adr/020-craftbook-adoption.md) D7).
1. **A domain or application crate never depends on a driver.** Drivers
   are adapters: they depend on the crate that owns the port and implement
   it ([ADR 019](adr/019-ports-in-domain-drivers-as-adapters.md)). Only the
   composition root (`sdk`, examples) names a concrete driver. Today's
   `graphics-core → graphics-driver` edge is pending migration (wave 5);
   audio (`audio-driver → audio-core`) and compute
   (`compute-driver → compute-runtime`) are done.

2. **A driver never depends on a different subsystem's driver or domain.**
   An adapter depends on the port crate of its own context and on the
   technology it wraps (`gpu-driver` for `wgpu`, `platform-core` for OS
   services). It knows the port's types and nothing else of the domain:
   scenes, materials and rigid bodies stay out of it.

3. **`meridian-ecs-core` depends on nothing above `meridian-gac-core` and
   `meridian-memory-core`.** It must never depend on `engine-core`,
   `graphics-core`, `physics-core`, `audio-core`, or `asset-core`. ECS is a
   storage mechanism (`Entity`, `Component`, `Archetype`, `Query`,
   `Storage`); it has no opinion about *gameplay/domain* component meaning
   (materials, rigid bodies, DSP graphs, ...) — that's what the
   `graphics-core`/`physics-core`/`audio-core` boundary above exists to
   enforce. The one exception is `Transform`, an engine-primitive spatial
   type built directly on `gac-core`'s `Motor3` and shared by every
   subsystem (see [gac-design.md](gac-design.md)); ECS knows about it the
   same way it knows about `Entity` itself, not as a domain concept it
   interprets.
4. **`meridian-asset-core` only loads and decodes.** It must never define a
   `AssetManager`, `ResourceManager`, or `CacheManager` type, and must never
   depend on `ecs-core`, `graphics-core`, or `engine-core`. Its job ends at
   "file bytes → decoder → CPU-side representation"; deciding where that
   representation lives and when it dies is the application's problem, not
   this crate's. Concretely: a loader function returns `CpuMeshData` (or
   equivalent), never a `MeshHandle`/`ResourceId` — minting a handle is
   `resource-core`'s job (rule 8), and an asset-core function that returns
   one would mean asset-core deciding runtime identity/lifetime, exactly
   what this rule exists to keep out of it.
5. **`meridian-compute-runtime` is the only path to CPU-SIMD/GPU-compute
   for subsystem crates.** `physics-core` and `graphics-core` reach compute
   through `compute-runtime` (directly, or via an adapter crate like
   `gac-compute` — see rule 10), never by depending on `compute-driver`
   directly (only adapter crates and the composition root do), and never
   by re-implementing scheduling/dispatch themselves.
   Any future `animation-core`, `particles-core`, or `ai-core` follows the
   same rule.
6. **`meridian-gac-core` stays pure geometric algebra.** Scalar types, SIMD
   dispatch, and CPU feature detection live in `meridian-numeric-core`
   (which itself sits on `meridian-foundation`), not in `gac-core`. If you
   find yourself adding a `#[cfg(target_feature = ...)]` block to
   `gac-core`, it belongs in `numeric-core` instead.
7. **`meridian-engine-core` is where cross-subsystem coordination lives.**
   If two subsystems need to talk outside the edges drawn above, that
   coordination is a use case in `engine-core`, not a new cross-dependency
   between domain crates. It is the runtime context's application layer,
   not a hub: per [ADR 020](adr/020-craftbook-adoption.md) D2 it stops
   owning subsystem state (wave 6).

8. **`meridian-resource-core` defines resource *identity*, not lifetime
   policy.** `Handle`, `ResourceId`, versioning, and dependency-tracking
   types live here; deciding when a resource is loaded, evicted, or
   reloaded does not. It must never define a manager type (same rule as
   asset-core, rule 4) and must depend on nothing but `memory-core`. See
   [ADR 006](adr/006-resource-core-separation.md).
9. **Physics has no driver crate.** The unused `meridian-physics-driver`
   (a capability report and a step counter nobody called) was deleted in
   wave 5 (ADR 019); `CpuCapabilities` already lives in `platform-core`.
   BVH construction, spatial hashing and broad-phase structures are domain
   logic in `physics-core` (see [physics-design.md](physics-design.md)).

10. **`meridian-gac-core` and `meridian-compute-runtime` never depend on
    each other, in either direction.** `gac-core` stays pure geometric
    algebra (rule 6) and must never know a GPU exists; `compute-runtime`
    stays a generic dispatch runtime (rule 5) and must never know what a
    `Motor3` is. Batch GAC operations that need both — `MotorTransformKernel`
    and friends — live in the adapter crate `meridian-gac-compute`, which
    depends on both and is the only crate allowed to. See
    [ADR 007](adr/007-batch-transforms-via-compute.md).
11. **No domain-specific GPU/SIMD algorithm may live in
    `meridian-compute-runtime`.** It owns dispatch mechanics only
    (`ComputeContext`, `ComputeKernel`, buffers, and *dispatch ordering
    within a compute submission* — not general engine task scheduling,
    which stays `task-core`'s job; see
    [threading-model.md](threading-model.md)) and must never grow a kernel
    that encodes what a `Motor3`, a particle, or a rigid body is. Every
    domain that needs batched compute gets its own
    `meridian-<domain>-compute` adapter crate — `gac-compute` and
    `physics-compute` today, a future `particle-compute`/`ai-compute` as
    they're needed — depending on that domain's `*-core` plus
    `compute-runtime`, per rule 10.
    This is what keeps `compute-runtime` small and stable as new domains
    adopt it instead of each one adding its own edge into its internals.
    The same boundary applies to memory, not just algorithms:
    `compute-runtime` owns untyped buffers (`Buffer`, byte length,
    dispatch/sync state); it must never define a domain-shaped buffer type
    (`MotorBuffer`, `ParticleBuffer`, ...). Interpreting bytes as a
    `Motor3` or a particle is the adapter crate's job — same split as rule 4
    (`asset-core` decodes, it doesn't manage) and rule 8 (`resource-core` is
    identity, not policy).

12. **`meridian-gpu-driver` owns `wgpu` device/buffer/shader/compute-pipeline
    mechanics only — not rendering policy, not compute-dispatch
    scheduling.** It must never know what a render pass, a swapchain, a
    `ComputeKernel`, or a scheduling threshold is. The adapters built on it
    are the only crates allowed to depend on it: `graphics-driver`,
    `compute-driver`, and the GPU kernel adapters `gac-compute` and
    `physics-compute` (which name its buffer/shader/pipeline types). See
    [ADR 011](adr/011-shared-gpu-driver-crate.md).

## How to check locally

```sh
./build.sh check-deps
# or directly:
python3 scripts/check_dependency_rules.py
# to eyeball the raw graph:
cargo tree --workspace --edges normal
```

`check-deps` is authoritative and exact — it fails on any edge missing
from `ALLOWED`/["All edges, exactly"](#all-edges-exactly) *and* on any
edge listed there that a `Cargo.toml` no longer actually has, so the two
can never silently drift apart. If `cargo tree` shows an edge that
`check-deps` doesn't complain about, it's already documented; if
`check-deps` fails, either the edge is a real violation (fix the
`Cargo.toml`) or a legitimate new edge (add it to `ALLOWED` and to this
document, in the same change).

See also: [architecture.md](architecture.md), [context-map.md](context-map.md),
[ADR 019](adr/019-ports-in-domain-drivers-as-adapters.md) (supersedes ADR 005).
