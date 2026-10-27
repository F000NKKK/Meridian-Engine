# Segment: core

Ownership note and backlog for the `team/core` segment. This file says who
owns which crates and what is open in them; implementation status itself
lives in [roadmap.md](../roadmap.md), and crate edges in
[dependency-rules.md](../dependency-rules.md). If this file and either of
those disagree, those win and this file gets fixed.

## Scope

Math, memory, resource identity, assets, ECS, scheduling, platform, the app
loop and the application-facing SDK, plus the generic compute dispatch layer
every domain adapter builds on.

| Crate | Tier | Role | Skill |
|---|---|---|---|
| `meridian-foundation` | 0 | errors, logging, crash reporting | docs-guardian |
| `meridian-memory-core` | 0 | pools, arenas, generational handles | resource-lifetime-keeper |
| `meridian-task-core` | 0 | job graph scheduler | engine-integration-keeper |
| `meridian-numeric-core` | 1 | scalars, `Fixed`, SIMD/CPU dispatch | gac-math-keeper |
| `meridian-resource-core` | 1 | typed ids, versions, dependency graph | resource-lifetime-keeper |
| `meridian-platform-core` | 1 | window, input, time, dylib, capabilities | engine-integration-keeper |
| `meridian-gac-core` | 2 | geometric algebra, shapes, `GaFlavor` | gac-math-keeper |
| `meridian-asset-core` | 2 | decoders (image, mesh, audio, shader) | resource-lifetime-keeper |
| `meridian-ecs-core` | 3 | archetype SoA world, queries | data-oriented-ecs-keeper |
| `meridian-compute-driver` | 3 | CPU/GPU dispatch backends | compute-driver-keeper |
| `meridian-compute-runtime` | 4 | `ComputeKernel`, hybrid dispatch | compute-driver-keeper |
| `meridian-gac-compute` | 5 | GA batch kernels, `fixed_wgsl` | compute-driver-keeper |
| `meridian-engine-core` | 8 | frame scheduler, events, orchestration | engine-integration-keeper |
| `meridian-sdk` | 9 | single app-facing entry point | engine-integration-keeper |

## Borders with other segments

- `team/rendering`: `gpu-driver`, `graphics-driver`, `graphics-core`. Core
  keeps `compute-driver` on top of `gpu-driver` but does not own `wgpu`
  device mechanics.
- `team/physics`: `physics-driver`, `physics-core`, `physics-compute`.
  Core owns the `GaFlavor` traits and `gac-core` shapes they are generic
  over; physics must reuse them, not copy them.
- `team/audio`: `audio-driver`, `audio-core`. Core owns the audio decoders
  in `asset-core`.
- `team/scripting`: `dsl-core`, `dsl-macros`, the `.mel` extension. Core
  owns `sdk::dsl` registration of built-in tags only as far as the SDK
  re-export surface goes.
- `team/ui-ux`: no crate yet. When a UI crate appears, input routing stays
  in `platform-core`, layout/widgets go to the new crate.
- `team/data`: persistence/serialization formats (Substrate-Data).
  Component serialization hooks in `ecs-core` are core's; the format is
  data's.
- `team/standards`: Craftbook. Core adopts its rules once published; see
  "Standards gaps" below.

## Snapshot (2026-10-27)

Numbers from `src/**/*.rs`, non-test `unwrap()`/`expect()` counted roughly.

| Crate | LoC | `#[test]` | unwrap/expect | `unsafe` |
|---|---:|---:|---:|---:|
| foundation | 645 | 5 | 8 | 0 |
| numeric-core | 519 | 10 | 0 | 0 |
| gac-core | 3048 | 45 | 0 | 0 |
| memory-core | 306 | 8 | 2 | 0 |
| resource-core | 283 | 9 | 0 | 0 |
| task-core | 318 | 6 | 12 | 0 |
| platform-core | 1086 | 14 | 1 | 26 (dylib FFI) |
| asset-core | 1826 | 30 | 33 | 0 |
| ecs-core | 578 | 12 | 10 | 0 |
| engine-core | 1257 | 16 | 19 | 0 |
| sdk | 1640 | 23 | 6 | 0 |

## Backlog

Ordered by what blocks game work first. Done items stay listed with the
change that closed them.

1. **Done: multi-component ECS queries.** `World::query_many` /
   `query_many_mut` over tuples of two to four `&T` / `&mut T`, safe via
   `HashMap::get_disjoint_mut` ([ecs-design.md](../ecs-design.md)).
2. **ECS filters.** `Option<&T>` and `Without<T>` elements for
   `query_many`.
3. **ECS change detection and entity commands.** Deferred
   spawn/despawn/insert during iteration, and per-column change ticks so
   rendering can skip unchanged instances (feeds roadmap "Scaling" item 1).
4. **Persistence hooks.** Roadmap "Scaling" item 5. Core side: component
   enumeration per entity (`World::component_types(entity)`) and a world
   snapshot API; the format and `meridian-scene` belong to `team/data`.
5. **Done: persistent worker pool.** `Scheduler` spawns its workers once;
   `run` no longer spawns threads (1-job graph ~210 µs -> ~0.35 µs, 1 000-job
   chain 1.1 ms -> 0.33 ms, `cargo bench -p meridian-task-core`). Work-stealing
   deques wait for a frame profile that shows queue contention.
6. **Asynchronous asset I/O.** `asset-core` decodes in-memory bytes only.
   File and network loading with tokio belongs in a loader layer above it
   (ADR 009), never inside the decoders.
7. **Compute buffer reuse.** `compute-runtime` reallocates GPU buffers and
   bind groups per dispatch (roadmap, deferred). Build it when a per-frame
   caller exists.
8. **Benchmarks.** `ecs-core` has criterion benches; add them for
   `task-core` and the `gac-compute` batch kernels so items 3, 5 and 7 are
   decided on numbers.

The Craftbook migration (ADR 019/020) owns the `engine-core`/`sdk` rework,
the driver direction and the `unwrap` cleanup; see `docs/adr/020-craftbook-adoption.md`.

## Standards

Code in this segment follows Craftbook (ADR 020). Check before pushing:
`python3 <craftbook>/tools/craftcheck check . --baseline .craftcheck-baseline`
and `./build.sh check-deps`. Baselines may only shrink.
