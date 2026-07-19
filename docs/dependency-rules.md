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
        |                        |        meridian-compute-core             |
        |                        |                  |                       |
        +----------+-------------+------------------+-----------------------+
                    |
   meridian-asset-core   meridian-graphics-core   meridian-physics-core   meridian-audio-core
                    |                |                    |                     |
                    +----------------+--------------------+---------------------+
                                               |
                                    meridian-engine-core
```

(Arrows point from a dependency to its dependent. `meridian-audio-core` also
depends on `meridian-gac-core` and `meridian-audio-driver`, omitted above for
readability — see each crate's own `Cargo.toml` for the exact edge list.)

## Rules

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
   `Storage`); it has no opinion about what the components mean.
4. **`meridian-asset-core` only loads and decodes.** It must never define a
   `AssetManager`, `ResourceManager`, or `CacheManager` type, and must never
   depend on `ecs-core`, `graphics-core`, or `engine-core`. Its job ends at
   "file bytes → decoder → CPU-side representation"; deciding where that
   representation lives and when it dies is the application's problem, not
   this crate's.
5. **`meridian-compute-core` is the only path to CPU-SIMD/GPU-compute for
   subsystem crates.** `physics-core` and `graphics-core` reach compute
   through `compute-core`, never by depending on `compute-driver` directly,
   and never by re-implementing scheduling/dispatch themselves. Any future
   `animation-core`, `particles-core`, or `ai-core` follows the same rule.
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

## How to check locally

```sh
cargo tree --workspace --edges normal
```

If an edge in that output doesn't appear in the diagram above, it's either a
missing rule in this document or a violation — resolve which one before
merging.

See also: [architecture.md](architecture.md), [ADR 005](adr/005-driver-core-separation.md).
