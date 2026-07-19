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
meridian-compute-runtime compute dispatch runtime (device/context, buffers, ComputeKernel), no algorithms
meridian-gac-compute     GAC batch kernels (Motor3 transforms) — adapter between gac-core and compute-runtime
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
`platform-core` have none, `engine-core` sits on top of everything.
`gac-core` (math) and `compute-runtime` (dispatch execution) are
deliberately independent of each other — neither depends on the other.
`gac-compute` is the adapter crate that depends on both, so batched `Motor3`
work can run on CPU-SIMD or GPU compute without `gac-core` ever knowing a
GPU exists (see [ADR 007](docs/adr/007-batch-transforms-via-compute.md)).
The full graph, and the rules for which direction a dependency is allowed to
point, are documented in
[`docs/dependency-rules.md`](docs/dependency-rules.md).

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
./release.sh meridian-gac-core --minor    # bump + cascade + publish
./release.sh meridian-engine-core --patch # no cascade, patch is link-compatible
./release.sh meridian-gac-core            # no bump: publish current version if not already on crates.io
./release.sh --publish-all --patch        # bump + publish every crate in the workspace
./release.sh --publish-all                # publish whatever isn't already on crates.io, no bump
```

`--publish-all` replaces `<crate-name>` and builds the plan from the whole
workspace (topologically) instead of one crate's cascade. A bump type
(`--patch`/`--minor`/`--major`) is always optional — omitting it (or passing
`--no-bump` explicitly) means "don't change the version," and for each crate
in the plan the script checks crates.io and publishes only what isn't
already there.

Before bumping a "round" version (patch bump is never round; a minor bump is
round when `patch == 0`; a major bump is round when `minor == 0 && patch ==
0`) the script checks whether that version is actually published — if not,
it publishes the current version as-is instead of skipping past an
unreleased one. `--no-check-ver` disables this and bumps/publishes blindly.

Add `--dry-run` to preview, `--no-publish` to bump without publishing.
