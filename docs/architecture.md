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
        |          |               |               |
  graphics-driver  physics-driver  audio-driver     |
        |          |               |               |
        +----------+-------+-------+---------------+
                            |
                     platform-core
                            |
                     meridian-foundation
```

`gac-core` (math) and `compute-runtime` (dispatch execution) are a second,
parallel spine underneath `graphics-core`/`physics-core`, deliberately
independent of each other:

```text
graphics-core   physics-core
        |             |
        +------+------+
               |
     meridian-gac-compute
        |             |
        v             v
meridian-gac-core   meridian-compute-runtime
        |                     |
meridian-numeric-core   meridian-compute-driver
        |                     |
meridian-foundation      platform-core
```

`gac-core` defines *what* to compute (`Motor3`, `Rotor`, ...); `compute-runtime`
defines *where* (CPU-SIMD or GPU, via `compute-driver`); `gac-compute` is the
adapter that implements `compute-runtime`'s `ComputeKernel` trait for batched
GAC operations. `graphics-core` and `physics-core` depend on `gac-compute`
for batched transform work and on `compute-runtime` directly for non-GAC
compute (e.g. GPU culling) — see
[ADR 007](adr/007-batch-transforms-via-compute.md). `gac-core` never depends
on `compute-runtime`, and `compute-runtime` never depends on `gac-core`, in
either direction.

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
domain-logic crate built on top of the driver. This is what lets a
`vulkan-driver` or `dx12-driver` be added later without touching
`graphics-core` at all. See [ADR 005](adr/005-driver-core-separation.md).
`compute` is named `compute-runtime` rather than `compute-core`: unlike
graphics/audio/physics, it has no domain concepts of its own (no algorithm
lives in it) — it's dispatch infrastructure that domain crates
(`gac-compute`, and future `particle-compute`/`physics-compute`/...) build
on top of, so it doesn't get the "core" name reserved for
`graphics-core`/`physics-core`/`audio-core`-style domain layers. See
[ADR 007](adr/007-batch-transforms-via-compute.md).

## Threading and scheduling

Frame work is expressed as a job graph with automatically-derived execution
order, not a hand-written sequence of system calls. See
[threading-model.md](threading-model.md).

## Current state

Every crate except the audio-device driver backend (blocked on the same
`Window`/`DynamicLibrary`-class OS-device decision as `platform-core::Window`,
see [roadmap.md](roadmap.md)) now has a real implementation, up through
`meridian-engine-core`: `Runtime` owns a `SubsystemManager` (real
`ecs-core`/`physics-core`/`audio-core` instances) and an `EventSystem`
(type-erased pub/sub, the mechanism rule 7 exists for — subsystems
communicate through it instead of depending on each other), and
`Runtime::tick` advances physics then recomputes audio from the result
each frame. `graphics-driver` has a real, headless `wgpu` `Device` (see
[roadmap.md](roadmap.md)'s `wgpu` entry), and `graphics-core`'s render
graph/camera/culling are real too, but neither is wired into
`Runtime::tick` yet — rendering has nothing to present a frame to without
a window/swapchain surface. See [roadmap.md](roadmap.md) for the exact
state of each crate and what's
still a scaffold.
