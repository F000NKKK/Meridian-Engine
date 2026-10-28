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

`meridian-diagnostics` and `meridian-memory` are the graph's
independent leaves — each depends on nothing (rule 0 covers `diagnostics`
specifically: *any* other crate may take an edge to it, so it isn't drawn
as a dependency of anything below).

The real graph has too many crisscrossing edges (many crates each
depend directly on `diagnostics`, `gac`, `resource`, ... —
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
tier 0  compute-app, diagnostics, markup-macros, memory, text-domain
tier 1  asset-domain, diagnostics-fs, markup, numeric, platform-winit, resource, ui-domain
tier 2  asset-app, asset-gltf, asset-jpeg, asset-png, asset-symphonia, gac, gpu-wgpu, ui-mel
tier 3  asset-fs, asset-http, audio-domain, compute-wgpu, ecs, physics-domain
tier 4  audio-app, gac-wgpu, rendering-domain, physics-app, data-substrate
tier 5  audio-cpal, editor-domain, runtime-app, physics-wgpu, rendering-app, ui-scene2d
tier 6  editor-app, rendering-wgpu, runtime-threads
tier 7  editor-tcp, sdk
```

`meridian-markup`/`meridian-markup-macros` are the two crates behind the
SDK's extensible scene DSL (see [ADR 015](adr/015-extensible-scene-dsl.md)
and `meridian-sdk::dsl`'s own module doc): `markup` is a domain-blind
tag/attribute-markup parser plus the generic `DslTag`/`DslRegistry`
machinery (tier 1: it depends only on `diagnostics`, for its
`EngineError` impls); `markup-macros` is the `#[dsl_tag(name = "...")]`
proc-macro that implements `DslTag` for a plain struct (tier 0: a
proc-macro crate, no meridian-* runtime dependency at all — it depends
on `syn`/`quote`/`proc-macro-crate` externally, not shown here since
this diagram only tracks internal `meridian-*` edges). Neither knows
about `Entity`/`Mesh`/`RigidBody` or any other specific tag — those are
defined in `meridian-sdk::dsl` (built-ins) or in a game's own crate
(custom tags), both just ordinary `#[dsl_tag]`-annotated structs.
`meridian-ui-mel` (tier 2, on `markup` and `ui-domain`; the latter only
to resolve a bound attribute to its UI property at build time) is the build-time
`.meui` -> `.g.rs` generator of [ADR 017](adr/017-meui-markup-and-code-behind.md).
An application takes it as a **build-dependency** for its `build.rs`,
the one exception to "applications depend on `meridian-sdk` alone": a
build script cannot use the SDK without compiling the whole engine for
the host, and the generated code itself names only `meridian_sdk`.

`meridian-data-substrate` is scene serialization on Substrate Data (see
[ADR 016](adr/016-data-formats-on-substrate-data.md) and
[data-formats.md](data-formats.md)): an application-built
`ComponentRegistry` and `.mscene` load/save into an `ecs::World`. It
depends on `ecs` (the `World` it fills) and on `gac` with its
optional `data` feature (`Motor3`'s schema, for `Transform`), plus the
external `substrate-data`. It sits *above* `ecs` on purpose, so
`ecs` stays file-blind (rule 3): `Transform` is stored through a
`TransformForm` newtype in `meridian-data-substrate` rather than `ecs` growing a
format dependency, and the scene's `Parent` component lives there too
until `ecs` has a hierarchy of its own. Nothing depends on it yet; `runtime-app`/`sdk` will when the
editor host and scene loading are wired up (docs/roadmap.md). The
`data` features of `numeric`/`gac` add no internal edge — only
the optional external `substrate-data` dependency.

`meridian-editor-domain` is the wire protocol between a running game and
its editor (Meridian IDLE): typed `Request`/`Response`/`Event` messages as
Substrate Data frames (see [editor-domain.md](editor-domain.md)). It
sits *above* `meridian-data-substrate` because it reuses its `ComponentName` and
carries `ComponentRegistry::describe()` unchanged, and takes `ecs` only
for `Entity`'s shape (`EntityRef`). Pure protocol: no `async`, no sockets,
no I/O beyond a generic `Read`/`Write`.

`meridian-editor-app` (tier 6) is the game's side of that protocol without
I/O: a `Session` that answers decoded requests against a `World` and a
`ComponentRegistry`, a `SimulationControl` port the frame loop implements,
and a `RequestHandler` port a transport calls. It sits above
`editor-domain`, `scene` and `ecs` and needs no edge beyond those;
it is *infra* in `craftbook.toml` because the protocol crate it speaks is
(an app crate may not depend on infra, ADR 019). `meridian-editor-tcp`
(tier 7) is its adapter: the only crate of the editor host with `tokio`,
serving one editor over loopback TCP with `substrate_data::io` frames.
Neither depends on `runtime-app`, and `runtime-app` does not depend on
them yet: the wiring is a follow-up ([editor-domain.md](editor-domain.md#host)).

`meridian-ui-domain` (tier 1, on `diagnostics` and `memory` only) is the
WPF-style UI runtime of [ADR 018](adr/018-ui-architecture.md): element
tree, typed properties, layout and hit testing. It deliberately has no
edge to `rendering-domain`, `platform-winit` or `ecs`: turning a laid-out
tree into `Scene2D` renderables and feeding it `InputState` are the job of
`meridian-ui-scene2d` (the adapter above both `ui-domain` and `rendering-domain`,
ADR 021) does the first half; the second is a later adapter above
`platform-winit`.

This shows *depth*, not *which crate depends on which* — two crates in
the same tier aren't necessarily related at all (`audio-domain` and
`ecs` share tier 3 but neither depends on the other). For the
actual edges, use ["All edges, exactly"](#all-edges-exactly); don't
infer them from tier adjacency.

`meridian-rendering-wgpu` and `meridian-compute-wgpu` both also depend
on `meridian-gpu-wgpu` (omitted from the diagram above for
readability, the same way the `gac-wgpu` edges below are) — the crate
that owns the actual `wgpu` device/buffer/shader/compute-pipeline
mechanics shared between rendering and general GPU compute, so neither
driver reimplements it independently or reaches into the other's crate
to get at it:

```text
        meridian-platform-winit
                   |
          meridian-gpu-wgpu
                   |
        +----------+----------+
        |                     |
rendering-wgpu          compute-wgpu
```

See [ADR 011](adr/011-shared-gpu-driver-crate.md) for the full decision.

`meridian-gac` and `meridian-compute-app` never depend on each
other — geometric algebra ("what to compute") and the compute dispatch
runtime ("where to compute it") are deliberately independent. The adapter
between them is its own crate:

```text
meridian-gac   meridian-compute-app
        |                     |
        +----------+----------+
                   |
         meridian-gac-wgpu
                    |
        +-----------+-----------+
        |                       |
  rendering-domain            physics-domain
```

`meridian-gac-wgpu` depends on both and implements `compute-app`'s
`ComputeKernel` trait for GAC batch operations (`MotorTransformKernel`,
`MotorComposeKernel`, ...). `rendering-domain` depends on
`gac-wgpu` for batched transform work and on `compute-app` directly
for non-GAC compute (e.g. GPU culling) — both edges omitted from the main
diagram above for readability. `physics-domain` has neither edge: its batching
lives in `physics-wgpu`. Also omitted: `meridian-audio-domain`'s
dependency on `meridian-gac`/
`meridian-resource` and `meridian-rendering-domain`'s/
`meridian-physics-domain`'s own direct dependency on `meridian-resource`
(handle types their `MeshRegistry`/`ColliderMeshHandle` etc. use) — see
["All edges, exactly"](#all-edges-exactly) for the complete, non-abridged
list.

`meridian-gac-wgpu` and `meridian-physics-wgpu` are adapters of the
compute context: they depend on `meridian-compute-wgpu` (the adapter that
implements `compute-app`'s `ComputeDevice` and `GpuBackend` ports) and on
`meridian-gpu-wgpu`. A kernel still dispatches through
`compute-app::ComputeContext`, never around it; it reaches the GPU with
`ComputeContext::gpu::<compute_driver::GpuComputeDevice>()`, whose
buffer/shader/pipeline methods
are typed in terms of `gpu-wgpu`'s own resource types
(`Buffer`/`BufferUsage`/`Shader`/`ComputePipeline`) — a real GPU-dispatching
`ComputeKernel` has to name those types (a bind-group's usage flags, a
kernel's own pipeline field, ...), which needs `gpu-wgpu` in scope. This
doesn't reach around `compute-app`'s dispatch mechanism (rule 5's
actual concern); it's the same category of edge `rendering-wgpu`/
`compute-wgpu` already have to `gpu-wgpu` for the same
resource-type-naming reason — see
[ADR 011](adr/011-shared-gpu-driver-crate.md). See
[ADR 007](adr/007-batch-transforms-via-compute.md).

`meridian-physics-wgpu` is rule 11's first concrete
`meridian-<domain>-compute` adapter: `physics-domain`'s deterministic
soft-body GPU kernels (mass-spring integration), mirroring
`gac-wgpu`'s shape one layer up:

```text
meridian-physics-domain   meridian-compute-app
        |                        |
        +-----------+------------+
                     |
          meridian-physics-wgpu
```

Same reasoning as `gac-wgpu` throughout: `physics-wgpu` depends
directly on `meridian-gpu-wgpu` for the same resource-type-naming
reason, and also on `meridian-gac` (the `GaFlavor`/`Plane`/`Vec3`
types its kernels' signatures are expressed in) and `meridian-gac-wgpu`
(its `Fixed`-flavor kernel reuses `fixed_wgsl::FIXED_ARITHMETIC_LIB_WGSL`
rather than re-deriving the same Q16.16 emulation — see
`meridian-physics-wgpu`'s own module doc). It also takes rule 0's
open `meridian-diagnostics` edge: `narrow_phase::GenerateContactsKernel`
logs via `log_warn!` when a pair's manifold exceeds its fixed
`MAX_CONTACTS_PER_PAIR` output slot, the same `MAX_LIGHTS`-style
truncation policy `rendering-domain::submission` uses.

`meridian-runtime-app` takes no edge to `physics-wgpu`, `runtime-threads` or
`platform-winit` ([ADR 025](adr/025-wave-6-runtime-and-composition-root.md)):
physics stepping is the `PhysicsStepper` port of `meridian-physics-app`,
implemented by `physics-wgpu`'s `BatchedStepper`; threads are the
`JobExecutor` port of `runtime-app`, implemented by `runtime-threads`'s
`TaskExecutor`. Only the composition root (`sdk`, examples) chooses which
implementation a `Runtime` gets.

## All edges, exactly

The ground truth: every crate's real `[dependencies]`, mirroring
`ALLOWED` in `scripts/check_dependency_rules.py` (`./build.sh
check-deps` fails if a real edge and this list disagree — regenerate
both together when either changes, in the same PR). Ordered roughly
bottom-up.

- `meridian-diagnostics` -> (nothing)
- `meridian-memory` -> (nothing)
- `meridian-runtime-threads` -> `meridian-asset-app`, `meridian-runtime-app`
- `meridian-numeric` -> `meridian-diagnostics`
- `meridian-platform-winit` -> `meridian-diagnostics`
- `meridian-gac` -> `meridian-numeric`
- `meridian-resource` -> `meridian-memory`
- `meridian-ecs` -> `meridian-gac`, `meridian-memory`
- `meridian-gpu-wgpu` -> `meridian-diagnostics`, `meridian-platform-winit`
- `meridian-rendering-app` -> `meridian-asset-domain`, `meridian-rendering-domain`, `meridian-memory`
- `meridian-rendering-wgpu` -> `meridian-asset-domain`, `meridian-diagnostics`, `meridian-gac`, `meridian-gpu-wgpu`, `meridian-rendering-domain`, `meridian-memory`, `meridian-platform-winit`, `meridian-rendering-app`
- `meridian-compute-wgpu` -> `meridian-compute-app`, `meridian-gpu-wgpu`, `meridian-platform-winit`
- `meridian-audio-app` -> `meridian-audio-domain`
- `meridian-audio-cpal` -> `meridian-audio-app`, `meridian-audio-domain`, `meridian-diagnostics`, `meridian-platform-winit`
- `meridian-compute-app` -> (none; it owns the `ComputeDevice` and `GpuBackend` ports)
- `meridian-gac-wgpu` -> `meridian-compute-wgpu`, `meridian-compute-app`, `meridian-gac`, `meridian-gpu-wgpu`, `meridian-numeric`
- `meridian-diagnostics-fs` -> `meridian-diagnostics`
- `meridian-asset-domain` -> `meridian-diagnostics`
- `meridian-asset-app` -> `meridian-asset-domain`
- `meridian-asset-fs` -> `meridian-asset-app`
- `meridian-asset-http` -> `meridian-asset-app`
- `meridian-asset-gltf` -> `meridian-asset-domain`, `meridian-gac`
- `meridian-asset-jpeg` -> `meridian-asset-domain`
- `meridian-asset-png` -> `meridian-asset-domain`
- `meridian-asset-symphonia` -> `meridian-asset-domain`
- `meridian-rendering-domain` -> `meridian-ecs`, `meridian-diagnostics`, `meridian-gac`, `meridian-memory`, `meridian-resource`
- `meridian-audio-domain` -> `meridian-gac`
- `meridian-physics-domain` -> `meridian-gac`, `meridian-numeric`, `meridian-resource`
- `meridian-physics-wgpu` -> `meridian-compute-wgpu`, `meridian-compute-app`, `meridian-diagnostics`, `meridian-gac-wgpu`, `meridian-gac`, `meridian-gpu-wgpu`, `meridian-numeric`, `meridian-physics-app`, `meridian-physics-domain`
- `meridian-runtime-app` -> `meridian-audio-app`, `meridian-compute-app`, `meridian-diagnostics`, `meridian-physics-app`
- `meridian-physics-app` -> `meridian-gac`, `meridian-physics-domain`
- `meridian-data-substrate` -> `meridian-ecs`, `meridian-gac`
- `meridian-editor-domain` -> `meridian-ecs`, `meridian-data-substrate`
- `meridian-editor-app` -> `meridian-ecs`, `meridian-editor-domain`, `meridian-data-substrate`
- `meridian-editor-tcp` -> `meridian-editor-app`, `meridian-editor-domain`
- `meridian-markup` -> `meridian-diagnostics`
- `meridian-markup-macros` -> (nothing; external `syn`/`quote`/`proc-macro-crate` only)
- `meridian-ui-mel` -> `meridian-markup`, `meridian-ui-domain`
- `meridian-ui-domain` -> `meridian-diagnostics`, `meridian-memory`
- `meridian-text-domain` -> (no workspace crates)
- `meridian-ui-scene2d` -> `meridian-gac`, `meridian-rendering-domain`, `meridian-text-domain`, `meridian-ui-domain`
- `meridian-sdk` -> `meridian-asset-app`, `meridian-asset-fs`, `meridian-asset-http`, `meridian-asset-domain`, `meridian-asset-gltf`, `meridian-asset-jpeg`, `meridian-asset-png`, `meridian-asset-symphonia`, `meridian-audio-app`, `meridian-audio-domain`, `meridian-audio-cpal`, `meridian-compute-wgpu`, `meridian-compute-app`, `meridian-diagnostics-fs`, `meridian-markup`, `meridian-markup-macros`, `meridian-runtime-app`, `meridian-diagnostics`, `meridian-gac`, `meridian-rendering-domain`, `meridian-rendering-wgpu`, `meridian-physics-app`, `meridian-physics-wgpu`, `meridian-physics-domain`, `meridian-platform-winit`, `meridian-rendering-app`, `meridian-runtime-threads`, `meridian-ui-domain`

`meridian-runtime-app` has no edge to `meridian-rendering-domain` or
`meridian-asset-domain`: `Runtime::tick` doesn't render a frame or load an
asset (see that crate's own module doc and docs/roadmap.md's
`rendering-domain`/`Runtime::tick` entry). Rule 7 would allow the edges the day
a frame step needs them; add them then, not before.

**`meridian-sdk` is the composition root** (interface layer,
[ADR 020](adr/020-craftbook-adoption.md) D3): the single application-facing
re-export surface and the one library crate that names concrete adapters
(`rendering-wgpu`, here, for windowed rendering). The interface layer may
depend on every layer; what it may not hold is business logic. Its
`scene`/`camera`/`render`/`dsl` logic is scheduled to move to the context
crates it belongs to (wave 6). Applications depend on `sdk` alone.

## Rules

0. **`meridian-diagnostics` is the open bottom of the graph.** It depends
   on nothing, and *any* crate may take an edge to it — that edge is added to
   `scripts/check_dependency_rules.py` when actually taken, not
   pre-declared. It exists for exactly two kinds of content: shared
   conventions (`EngineError`, `FeatureFlags`) and process-wide
   diagnostics (the `logging` kernel and its `LogSink` port). Diagnostics are deliberately *not* a "global manager" in rule
   3's sense: the sink owns no engine objects and hands out no handles —
   it appends lines, the same category as `std`'s own panic hook. It is
   the one global exempted in `craftbook.toml`; the log file and the crash
   report are the adapter `meridian-diagnostics-fs`, installed by the
   composition root ([ADR 020](adr/020-craftbook-adoption.md) D7).
1. **A domain or application crate never depends on a driver.** Drivers
   are adapters: they depend on the crate that owns the port and implement
   it ([ADR 019](adr/019-ports-in-domain-drivers-as-adapters.md)). Only the
   composition root (`sdk`, examples) names a concrete driver. Audio (`audio-cpal → audio-app`), compute
   (`compute-wgpu → compute-app`), rendering
   (`rendering-wgpu → rendering-app`) and assets (`asset-png`/`asset-symphonia`
   → `asset-domain`) are inverted ([ADR 024](adr/024-wave-5-ports-and-adapters.md)).

2. **A driver never depends on a different subsystem's driver or domain.**
   An adapter depends on the port crate of its own context and on the
   technology it wraps (`gpu-wgpu` for `wgpu`, `platform-winit` for OS
   services). It knows the port's types and nothing else of the domain:
   scenes, materials and rigid bodies stay out of it.

3. **`meridian-ecs` depends on nothing above `meridian-gac` and
   `meridian-memory`.** It must never depend on `runtime-app`,
   `rendering-domain`, `physics-domain`, `audio-domain`, or `asset-domain`. ECS is a
   storage mechanism (`Entity`, `Component`, `Archetype`, `Query`,
   `Storage`); it has no opinion about *gameplay/domain* component meaning
   (materials, rigid bodies, DSP graphs, ...) — that's what the
   `rendering-domain`/`physics-domain`/`audio-domain` boundary above exists to
   enforce. The one exception is `Transform`, an engine-primitive spatial
   type built directly on `gac`'s `Motor3` and shared by every
   subsystem (see [gac-design.md](gac-design.md)); ECS knows about it the
   same way it knows about `Entity` itself, not as a domain concept it
   interprets.
4. **`meridian-asset-domain` only loads and decodes.** It must never define a
   `AssetManager`, `ResourceManager`, or `CacheManager` type, and must never
   depend on `ecs`, `rendering-domain`, or `runtime-app`. Its job ends at
   "file bytes → decoder → CPU-side representation"; deciding where that
   representation lives and when it dies is the application's problem, not
   this crate's. Concretely: a loader function returns `CpuMeshData` (or
   equivalent), never a `MeshHandle`/`ResourceId` — minting a handle is
   `resource`'s job (rule 8), and an asset-domain function that returns
   one would mean asset-domain deciding runtime identity/lifetime, exactly
   what this rule exists to keep out of it.
5. **`meridian-compute-app` is the only path to CPU-SIMD/GPU-compute
   for subsystem crates.** `physics-domain` and `rendering-domain` reach compute
   through `compute-app` (directly, or via an adapter crate like
   `gac-wgpu` — see rule 10), never by depending on `compute-wgpu`
   directly (only adapter crates and the composition root do), and never
   by re-implementing scheduling/dispatch themselves.
   Any future `animation-core`, `particles-core`, or `ai-core` follows the
   same rule.
6. **`meridian-gac` stays pure geometric algebra.** Scalar types, SIMD
   dispatch, and CPU feature detection live in `meridian-numeric`
   (which itself sits on `meridian-diagnostics`), not in `gac`. If you
   find yourself adding a `#[cfg(target_feature = ...)]` block to
   `gac`, it belongs in `numeric` instead.
7. **`meridian-runtime-app` is where cross-subsystem coordination lives.**
   If two subsystems need to talk outside the edges drawn above, that
   coordination is a use case in `runtime-app`, not a new cross-dependency
   between domain crates. It is the runtime context's application layer,
   not a hub: per [ADR 020](adr/020-craftbook-adoption.md) D2 and
   [ADR 025](adr/025-wave-6-runtime-and-composition-root.md) it owns no
   subsystem state and names no adapter.

8. **`meridian-resource` defines resource *identity*, not lifetime
   policy.** `Handle`, `ResourceId`, versioning, and dependency-tracking
   types live here; deciding when a resource is loaded, evicted, or
   reloaded does not. It must never define a manager type (same rule as
   asset-domain, rule 4) and must depend on nothing but `memory`. See
   [ADR 006](adr/006-resource-core-separation.md).
9. **Physics has no driver crate.** The unused `meridian-physics-driver`
   (a capability report and a step counter nobody called) was deleted in
   wave 5 (ADR 019); `CpuCapabilities` already lives in `platform-winit`.
   BVH construction, spatial hashing and broad-phase structures are domain
   logic in `physics-domain` (see [physics-design.md](physics-design.md)).

10. **`meridian-gac` and `meridian-compute-app` never depend on
    each other, in either direction.** `gac` stays pure geometric
    algebra (rule 6) and must never know a GPU exists; `compute-app`
    stays a generic dispatch runtime (rule 5) and must never know what a
    `Motor3` is. Batch GAC operations that need both — `MotorTransformKernel`
    and friends — live in the adapter crate `meridian-gac-wgpu`, which
    depends on both and is the only crate allowed to. See
    [ADR 007](adr/007-batch-transforms-via-compute.md).
11. **No domain-specific GPU/SIMD algorithm may live in
    `meridian-compute-app`.** It owns dispatch mechanics only
    (`ComputeContext`, `ComputeKernel`, buffers, and *dispatch ordering
    within a compute submission* — not general engine task scheduling,
    which stays `runtime-threads`'s job; see
    [threading-model.md](threading-model.md)) and must never grow a kernel
    that encodes what a `Motor3`, a particle, or a rigid body is. Every
    domain that needs batched compute gets its own
    `meridian-<domain>-compute` adapter crate — `gac-wgpu` and
    `physics-wgpu` today, a future `particle-compute`/`ai-compute` as
    they're needed — depending on that domain's `*-core` plus
    `compute-app`, per rule 10.
    This is what keeps `compute-app` small and stable as new domains
    adopt it instead of each one adding its own edge into its internals.
    The same boundary applies to memory, not just algorithms:
    `compute-app` owns untyped buffers (`Buffer`, byte length,
    dispatch/sync state); it must never define a domain-shaped buffer type
    (`MotorBuffer`, `ParticleBuffer`, ...). Interpreting bytes as a
    `Motor3` or a particle is the adapter crate's job — same split as rule 4
    (`asset-domain` decodes, it doesn't manage) and rule 8 (`resource` is
    identity, not policy).

12. **`meridian-gpu-wgpu` owns `wgpu` device/buffer/shader/compute-pipeline
    mechanics only — not rendering policy, not compute-dispatch
    scheduling.** It must never know what a render pass, a swapchain, a
    `ComputeKernel`, or a scheduling threshold is. The adapters built on it
    are the only crates allowed to depend on it: `rendering-wgpu`,
    `compute-wgpu`, and the GPU kernel adapters `gac-wgpu` and
    `physics-wgpu` (which name its buffer/shader/pipeline types). See
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
