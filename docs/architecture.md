# Architecture overview

Meridian-Engine is a modular Rust computation platform for interactive
systems (games, CAD/CAE, simulators, visualizers, XR). It is not "another
Unity" — the goal is a minimal, stable core where every subsystem shares one
spatial model instead of each inventing its own transform representation.

## Layering

```text
                    Application Layer
        (game / editor / simulation / CAD / UI framework)
                            |
                     meridian-engine-core
                            |
        +----------+-------+-------+----------+
        |          |               |          |
  graphics-core physics-core  audio-core  asset-core / ecs-core
        |          |               |
  graphics-driver  physics-driver  audio-driver
        |          |               |
        +----------+-------+-------+
                            |
                     platform-core          compute-core --------+
                                                    |             |
                                             compute-driver  meridian-gac-core
                                                    |             |
                                             platform-core   meridian-numeric-core
                                                                  |
                                                          meridian-foundation
```

`graphics-core`, `physics-core` and future `animation-core`/`particles-core`/
`ai-core` reach batched CPU-SIMD/GPU work through `compute-core`, which in
turn depends on `gac-core` for the `Motor3` type those batches operate on —
see [ADR 007](adr/007-batch-transforms-via-compute.md). `gac-core` never
depends on `compute-core` back; it stays pure geometric algebra regardless
of which backend eventually executes a given batch.

See [dependency-rules.md](dependency-rules.md) for the exact, enforceable
edge list — this diagram is the intuition, that document is the ruling.

## Why geometric algebra

Classic engines represent transforms differently per subsystem: graphics
uses 4x4 matrices, physics uses its own position/orientation pair, audio
uses yet another spatial convention. Meridian-Engine instead gives every
subsystem the same `Motor3` (translation + rotation, composable, no gimbal
lock, no separate quaternion/vector pair to keep in sync). `gac-core` is the
one place that owns this — see [gac-design.md](gac-design.md) and
[ADR 001](adr/001-geometric-algebra-as-spatial-model.md).

## Why data-oriented ECS

Components are stored as Structure-of-Arrays per archetype, not as
per-entity structs. This is what makes SIMD-friendly, cache-friendly
iteration possible over thousands of transforms/velocities/meshes at once.
See [ecs-design.md](ecs-design.md) and
[ADR 004](adr/004-data-oriented-design.md).

## Why handle-based resources instead of `Arc<T>`

Textures, meshes and buffers are referenced by a generational `Handle {
index, generation }`, not by reference-counted pointers. This avoids
lifetime coupling across subsystem boundaries and makes resource pools
serializable. See [memory-model.md](memory-model.md) and
[ADR 002](adr/002-handle-based-resources.md).

## Why no global managers

There is deliberately no `AssetManager`, `ResourceManager`, or singleton
`Engine::instance()`. Ownership and lifetime decisions are made by the
application layer, on top of the core crates — see
[ADR 003](adr/003-no-global-managers.md).

## Driver / core split

Every hardware-facing subsystem (graphics, audio, physics, compute) splits
into a `*-driver` crate (hardware abstraction, no domain concepts) and a
`*-core` crate (domain logic, built on the driver). This is what lets a
`vulkan-driver` or `dx12-driver` be added later without touching
`graphics-core` at all. See [ADR 005](adr/005-driver-core-separation.md).

## Threading and scheduling

Frame work is expressed as a job graph with automatically-derived execution
order, not a hand-written sequence of system calls. See
[threading-model.md](threading-model.md).

## Current state

As of this writing, every crate in the workspace is a scaffold: a
`Cargo.toml` with the correct dependency edges and a one-line doc comment in
`lib.rs`. The priority before writing implementations is keeping this
document, [dependency-rules.md](dependency-rules.md), and the ADRs accurate
— see [roadmap.md](roadmap.md) for what comes next.
