# Meridian-Engine

A modular Rust engine core for real-time graphics, audio and physics, built
around a single geometric-algebra spatial layer instead of separate matrix /
transform conventions per subsystem.

Rust 1.92+ (edition 2024). Licensed under [MPL-2.0](LICENSE).

## Crates

```text
meridian-foundation      zero-dependency shared primitives (errors, feature detection)
meridian-numeric-core    scalar types, SIMD helpers, numeric traits
meridian-gac-core        geometric algebra: vectors, rotors, motors, transforms
meridian-memory-core     arenas, resource pools, generational handles
meridian-resource-core   typed resource handles, versioning, dependency tracking
meridian-task-core       job graph scheduler
meridian-platform-core   window, input, filesystem, time, threading
meridian-compute-driver  low-level CPU-SIMD / GPU-compute dispatch abstraction
meridian-compute-core    shared compute scheduling for physics/graphics/...
meridian-asset-core      image/mesh/audio/shader loading & decoding
meridian-ecs-core        archetype ECS, SoA storage
meridian-graphics-driver low-level GPU device abstraction
meridian-audio-driver    low-level audio device abstraction
meridian-physics-driver  low-level broad-phase / spatial structures
meridian-graphics-core   render graph, culling, lighting, materials, camera
meridian-physics-core    broad/narrow phase collision, constraint solver
meridian-audio-core      spatial mixer, DSP graph, listener/emitter
meridian-engine-core     runtime: frame scheduler, events, subsystem manager
```

Dependencies flow bottom-up: `foundation` / `memory-core` / `task-core` /
`platform-core` have none, `engine-core` sits on top of everything. The full
graph, and the rules for which direction a dependency is allowed to point,
are documented in [`docs/dependency-rules.md`](docs/dependency-rules.md).

## Documentation

- [`docs/architecture.md`](docs/architecture.md) — layering and the "why"
  behind the big decisions
- [`docs/dependency-rules.md`](docs/dependency-rules.md) — the enforceable
  rules for which crate may depend on which
- [`docs/gac-design.md`](docs/gac-design.md),
  [`docs/ecs-design.md`](docs/ecs-design.md),
  [`docs/graphics-design.md`](docs/graphics-design.md),
  [`docs/physics-design.md`](docs/physics-design.md),
  [`docs/memory-model.md`](docs/memory-model.md),
  [`docs/threading-model.md`](docs/threading-model.md) — per-subsystem design
- [`docs/roadmap.md`](docs/roadmap.md) — current state and implementation
  order
- [`docs/adr/`](docs/adr/) — architecture decision records

## Building

```sh
./build.sh build              # whole workspace
./build.sh test                # cargo test --workspace
./build.sh check-deps          # verify the crate graph matches docs/dependency-rules.md
./build.sh run hello_engine -- --foo bar   # run an example, forwarding args
./build.sh list-examples
./build.sh clean
```

See `examples/examples/` for example programs (standard Cargo `[[example]]`
convention).

## Releasing

```sh
./release.sh meridian-gac-core --minor          # bump + cascade + publish
./release.sh meridian-engine-core --patch       # no cascade, patch is link-compatible
./release.sh meridian-gac-core --publish-only   # publish current version as-is
./release.sh --publish-all --patch              # bump + publish every crate in the workspace
./release.sh --publish-all --no-bump            # publish every crate at its current version
```

`--publish-all` replaces `<crate-name>` and builds the plan from the whole
workspace (topologically) instead of one crate's cascade. `--no-bump` skips
the version bump and publishes the plan as-is — works with a single crate,
its cascade, or `--publish-all`.

Add `--dry-run` to preview, `--no-publish` to bump without publishing.
