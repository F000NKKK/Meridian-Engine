# Dependency rules

The workspace graph is a DAG on purpose. This document is the ruling
reference when a PR (human or agent-authored) is unsure which direction a
`use` or a `Cargo.toml` dependency is allowed to point.

## The graph

```text
meridian-foundation
        |
        v
meridian-numeric-core
        |
        v
meridian-gac-core   meridian-memory-core   meridian-task-core   meridian-platform-core
        |                  |     |                 |                     |
        |                  |     v                 |                     +---------------+
        |                  |  meridian-resource-core|                     |               |
        v                  v     |                  v                     v               v
meridian-ecs-core <--------+     |        meridian-compute-driver   meridian-graphics-driver
        |                        |                  |               meridian-audio-driver
        |                        |                  v               meridian-physics-driver
        |                        |        meridian-compute-runtime          |
        |                        |                                          |
        +----------+-------------+------------------------------------------+
                    |
   meridian-asset-core   meridian-graphics-core   meridian-physics-core   meridian-audio-core
                    |                |                    |                     |
                    +----------------+--------------------+---------------------+
                                               |
                                    meridian-engine-core
```

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
`MotorComposeKernel`, ...). `graphics-core` and `physics-core` depend on
`gac-compute` for batched transform work and on `compute-runtime` directly
for non-GAC compute (e.g. GPU culling) — both edges omitted from the main
diagram above for readability, along with `meridian-audio-core`'s dependency
on `meridian-gac-core` and `meridian-audio-driver`; see each crate's own
`Cargo.toml` for the exact edge list.

`meridian-gac-compute` also depends directly on `meridian-gpu-driver` —
not `meridian-compute-driver` (still forbidden by rule 5: a kernel
dispatches through `compute-runtime::ComputeContext`, never around it).
`compute-runtime::ComputeContext::gpu()` returns a
`compute-driver::GpuComputeDevice`, whose buffer/shader/pipeline methods
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
`meridian-physics-compute`'s own module doc).

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
   it appends lines, the same category as `std`'s own panic hook.
1. **A `*-core` crate never depends on the `*-driver` crate of a *different*
   subsystem.** `graphics-core` may depend on `graphics-driver`, never on
   `audio-driver` or `physics-driver`.
2. **A `*-driver` crate never depends on its own `*-core`.** Drivers are
   hardware/OS abstractions; they know nothing about scenes, materials,
   rigid bodies, or any other high-level concept their `*-core` builds on
   top of them. `graphics-driver → graphics-core` is forbidden.
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
   directly, and never by re-implementing scheduling/dispatch themselves.
   Any future `animation-core`, `particles-core`, or `ai-core` follows the
   same rule.
6. **`meridian-gac-core` stays pure geometric algebra.** Scalar types, SIMD
   dispatch, and CPU feature detection live in `meridian-numeric-core`
   (which itself sits on `meridian-foundation`), not in `gac-core`. If you
   find yourself adding a `#[cfg(target_feature = ...)]` block to
   `gac-core`, it belongs in `numeric-core` instead.
7. **`meridian-engine-core` is the only crate allowed to depend on every
   `*-core`.** No other crate is the "hub" — if two `*-core` crates need to
   talk to each other outside the edges drawn above, that coordination
   belongs in `engine-core`, not in a new cross-dependency between them.
8. **`meridian-resource-core` defines resource *identity*, not lifetime
   policy.** `Handle`, `ResourceId`, versioning, and dependency-tracking
   types live here; deciding when a resource is loaded, evicted, or
   reloaded does not. It must never define a manager type (same rule as
   asset-core, rule 4) and must depend on nothing but `memory-core`. See
   [ADR 006](adr/006-resource-core-separation.md).
9. **`meridian-physics-driver` owns no collision algorithms.** BVH
   construction, spatial hashing, and broad-phase structures are domain
   logic and belong in `physics-core`, even though they sound "low-level."
   `physics-driver` is execution only: memory backend, SIMD/GPU dispatch,
   synchronization — the same role `compute-driver` plays for compute in
   general. See [physics-design.md](physics-design.md).
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
    `meridian-<domain>-compute` adapter crate — `gac-compute` today,
    `particle-compute`/`physics-compute`/`ai-compute` as they're needed —
    depending on that domain's `*-core` plus `compute-runtime`, per rule 10.
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
    `ComputeKernel`, or a scheduling threshold is; `graphics-driver` and
    `compute-driver` (the only two crates allowed to depend on it) each
    add their own domain-specific layer on top. See
    [ADR 011](adr/011-shared-gpu-driver-crate.md).

## How to check locally

```sh
cargo tree --workspace --edges normal
```

If an edge in that output doesn't appear in the diagram above, it's either a
missing rule in this document or a violation — resolve which one before
merging.

See also: [architecture.md](architecture.md), [ADR 005](adr/005-driver-core-separation.md).
