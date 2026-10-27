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
| `meridian-diagnostics` | 0 | errors, logging, crash reporting | docs-guardian |
| `meridian-memory` | 0 | pools, arenas, generational handles | resource-lifetime-keeper |
| `meridian-runtime-threads` | 0 | job graph scheduler | engine-integration-keeper |
| `meridian-numeric` | 1 | scalars, `Fixed`, SIMD/CPU dispatch | gac-math-keeper |
| `meridian-resource` | 1 | typed ids, versions, dependency graph | resource-lifetime-keeper |
| `meridian-platform-winit` | 1 | window, input, time, dylib, capabilities | engine-integration-keeper |
| `meridian-gac` | 2 | geometric algebra, shapes, `GaFlavor` | gac-math-keeper |
| `meridian-asset-domain` | 2 | decoders (image, mesh, audio, shader) | resource-lifetime-keeper |
| `meridian-ecs` | 3 | archetype SoA world, queries | data-oriented-ecs-keeper |
| `meridian-compute-wgpu` | 3 | CPU/GPU dispatch backends | compute-driver-keeper |
| `meridian-compute-app` | 4 | `ComputeKernel`, hybrid dispatch | compute-driver-keeper |
| `meridian-gac-wgpu` | 5 | GA batch kernels, `fixed_wgsl` | compute-driver-keeper |
| `meridian-runtime-app` | 8 | frame scheduler, events, orchestration | engine-integration-keeper |
| `meridian-sdk` | 9 | single app-facing entry point | engine-integration-keeper |

## Borders with other segments

- `team/rendering`: `gpu-wgpu`, `rendering-wgpu`, `rendering-domain`. Core
  keeps `compute-wgpu` on top of `gpu-wgpu` but does not own `wgpu`
  device mechanics.
- `team/physics`: `physics-domain`, `physics-wgpu`.
  Core owns the `GaFlavor` traits and `gac` shapes they are generic
  over; physics must reuse them, not copy them.
- `team/audio`: `audio-cpal`, `audio-domain`. Core owns the audio decoders
  in `asset-domain`.
- `team/scripting`: `markup`, `markup-macros`, the `.meui` extension. Core
  owns `sdk::dsl` registration of built-in tags only as far as the SDK
  re-export surface goes.
- `team/ui-ux`: no crate yet. When a UI crate appears, input routing stays
  in `platform-winit`, layout/widgets go to the new crate.
- `team/data`: persistence/serialization formats (Substrate-Data).
  Component serialization hooks in `ecs` are core's; the format is
  data's.
- `team/standards`: Craftbook. Core adopts its rules once published; see
  "Standards gaps" below.

## Snapshot (2026-10-27)

Numbers from `src/**/*.rs`, non-test `unwrap()`/`expect()` counted roughly.

| Crate | LoC | `#[test]` | unwrap/expect | `unsafe` |
|---|---:|---:|---:|---:|
| foundation | 645 | 5 | 8 | 0 |
| numeric | 519 | 10 | 0 | 0 |
| gac | 3048 | 45 | 0 | 0 |
| memory | 306 | 8 | 2 | 0 |
| resource | 283 | 9 | 0 | 0 |
| runtime-threads | 318 | 6 | 12 | 0 |
| platform-winit | 1086 | 14 | 1 | 26 (dylib FFI) |
| asset-domain | 1826 | 30 | 33 | 0 |
| ecs | 578 | 12 | 10 | 0 |
| runtime-app | 1257 | 16 | 19 | 0 |
| sdk | 1640 | 23 | 6 | 0 |

## Backlog

Ordered by what blocks game work first. Done items stay listed with the
change that closed them.

1. **Done: multi-component ECS queries.** `World::query_many` /
   `query_many_mut` over tuples of two to four `&T` / `&mut T`, safe via
   `HashMap::get_disjoint_mut` ([ecs-design.md](../ecs-design.md)).
2. **Done: ECS filters.** `Option<&T>`, `Option<&mut T>`, `With<T>` and
   `Without<T>` elements for `query_many`/`query_many_mut`; the column pass
   no longer needs `get_disjoint_mut` (`query_many_mut` bench ~31 µs -> ~25 µs).
3. **ECS change detection and entity commands.** Change detection is
   **done**: a world clock and per-archetype-column ticks with a `Changed<T>`
   filter, so rendering can skip unchanged instance batches (feeds roadmap
   "Scaling" item 1). Deferred `Commands` (spawn/despawn/insert/remove
   recorded during iteration, applied after) are **done** too.
4. **Done: persistence hooks.** Roadmap "Scaling" item 5. Core side:
   component enumeration per entity (`World::component_types(entity)`, sorted
   `TypeId`s; the stable-name registry and serializers are the adapter's) and
   an in-memory world snapshot (`World::snapshot(&Cloners)` / `restore`, with
   entity generations; [ecs-design.md](../ecs-design.md#snapshots)). Agreed with
   `team/data`: the on-disk format and `meridian-data-substrate` stay theirs
   and need nothing more from `ecs`.
5. **Done: persistent worker pool.** `Scheduler` spawns its workers once;
   `run` no longer spawns threads (1-job graph ~210 µs -> ~0.35 µs, 1 000-job
   chain 1.1 ms -> 0.33 ms, `cargo bench -p meridian-runtime-threads`). Work-stealing
   deques wait for a frame profile that shows queue contention.
6. **Done: background asset loading** ([ADR 027](../adr/027-background-asset-loading.md)).
   `asset-app` (`ByteSource`, `Spawn`, `AssetLoader`, `Pending`), `asset-fs`
   (`FsSource`) and `runtime-threads::BackgroundPool`; a load returns at once
   and costs ~2.5 µs per small file on 4 threads (`cargo bench -p
   meridian-asset-fs`). Wired into `sdk` (`asset_loader`, `default_asset_loader`, `load_image`, loader
   types re-exported). Priority (`Low`/`Normal`/`High`) and
   `Pending::cancel` are in. `asset-http` (`HttpSource`, plain `http` only,
   [ADR 028](../adr/028-http-asset-source.md)) is the network source. Open: `https`,
   which waits for a decision on the `ring` licence.
7. **Compute buffer reuse.** `compute-app` reallocates GPU buffers and
   bind groups per dispatch (roadmap, deferred). Build it when a per-frame
   caller exists.
9. **Done: `Motor3` point transform fast path.** `transform_point` /
   `transform_vector` / `to_mat4` use the hand-expanded sandwich (rotor 3x3
   plus origin) instead of two 16-component products: 1.16 µs -> 8 ns per
   point, 10k points 5.6 ms -> 82 µs (`cargo bench -p meridian-gac`).
   `FixedMotor3` has the same expansion (`transform_point` 1.50 µs -> 13 ns,
   `transform_vector` 2.58 µs -> 10 ns); its results can differ from the old
   product by a few units of the last place and stay bit-identical across
   platforms.
8. **Benchmarks.** `ecs` has criterion benches; add them for
   `runtime-threads` and the `gac-wgpu` batch kernels so items 3, 5 and 7 are
   decided on numbers.

The Craftbook migration (ADR 019/020) owns the `runtime-app`/`sdk` rework,
the driver direction and the `unwrap` cleanup; see `docs/adr/020-craftbook-adoption.md`.

## Standards

Code in this segment follows Craftbook (ADR 020). Check before pushing:
`python3 <craftbook>/tools/craftcheck check . --baseline .craftcheck-baseline`
and `./build.sh check-deps`. Baselines may only shrink.
