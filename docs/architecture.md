# Architecture overview

Meridian-Engine is a modular Rust computation platform for interactive
systems (games, CAD/CAE, simulators, visualizers, XR). It is not "another
Unity" — the goal is a minimal, stable core where every subsystem shares one
spatial model instead of each inventing its own transform representation.

## Layering

The workspace follows Craftbook's domain-driven layering
([ADR 018](adr/018-ports-in-domain-drivers-as-adapters.md),
[ADR 019](adr/019-craftbook-adoption.md)): dependencies point inwards, and
the domain and application crates own the ports their adapters implement.

```text
  interface       sdk (composition root), examples
      |
  application     engine-core (runtime use cases), compute-runtime
      |
  domain          graphics-core, physics-core, audio-core, asset-core
      ^
  infrastructure  graphics-driver, audio-driver, compute-driver, gpu-driver,
                  gac-compute, physics-compute, platform-core, task-core
                  (adapters: implement the ports of the layers above)

  shared kernel   foundation, numeric-core, gac-core, memory-core,
                  resource-core, ecs-core, dsl-core  (used by every layer)
```

That is the target. Today several domain and application crates still
depend on their drivers (the ADR 005 direction); those edges are listed as
*migration pending* in `scripts/check_dependency_rules.py` and in
[dependency-rules.md](dependency-rules.md), and wave 5 of the Craftbook
migration inverts them one subsystem at a time. Layers and contexts per
crate: `craftbook.toml` and [context-map.md](context-map.md).

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
meridian-foundation      meridian-gpu-driver
                               |
                        platform-core
```

`gac-core` defines *what* to compute (`Motor3`, `Rotor`, ...); `compute-runtime`
defines *where* (CPU-SIMD or GPU, via `compute-driver`); `gac-compute` is the
adapter that implements `compute-runtime`'s `ComputeKernel` trait for batched
GAC operations, and also depends on `gpu-driver` directly for the GPU
resource types (`Buffer`, `Shader`, `ComputePipeline`, ...) a real
GPU-dispatching kernel has to name — see
[dependency-rules.md](dependency-rules.md) for why that's not the same as
depending on `compute-driver` directly (still forbidden). `graphics-core`
and `physics-core` depend on `gac-compute` for batched transform work and
on `compute-runtime` directly for non-GAC compute (e.g. GPU culling) —
see [ADR 007](adr/007-batch-transforms-via-compute.md). `gac-core` never
depends on `compute-runtime`, and `compute-runtime` never depends on
`gac-core`, in either direction. `compute-driver` and `graphics-driver`
both depend on `gpu-driver`, the crate that owns the actual `wgpu`
device/buffer/shader/compute-pipeline mechanics shared between rendering
and general GPU compute, so neither reimplements it independently — see
[ADR 011](adr/011-shared-gpu-driver-crate.md).

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

## Ports and adapters

Every hardware-facing subsystem (graphics, audio, compute) keeps a domain
crate and a driver crate, but the driver is an adapter: the domain (or its
application crate) declares the port it needs — a mesh store, a frame
recorder, an audio sink, a compute device — and the driver implements it.
A second backend is one more adapter implementing the same ports, with no
change to the domain. See
[ADR 018](adr/018-ports-in-domain-drivers-as-adapters.md), which supersedes
[ADR 005](adr/005-driver-core-separation.md)'s `*-core → *-driver`
direction; until wave 5 the old edges remain as pending migration.
`compute` is named `compute-runtime` rather than `compute-core`: unlike
graphics/audio/physics, it has no domain concepts of its own (no algorithm
lives in it) — it's dispatch infrastructure that domain crates
(`gac-compute` and `physics-compute` today, a future `particle-compute`/...)
build on top of, so it doesn't get the "core" name reserved for
`graphics-core`/`physics-core`/`audio-core`-style domain layers. See
[ADR 007](adr/007-batch-transforms-via-compute.md).

`graphics-driver` and `compute-driver` both need the same underlying
`wgpu` device/buffer/shader/compute-pipeline mechanics (rendering and
general GPU compute are both "do something with a GPU device," even
though what they submit to it differs completely) — that shared
mechanics lives in a third crate, `meridian-gpu-driver`, rather than
being duplicated in both or reached through one from the other (which
would cross the "no driver depends on a different subsystem's driver"
boundary). See [ADR 011](adr/011-shared-gpu-driver-crate.md).

## Threading and scheduling

Frame work is expressed as a job graph with automatically-derived execution
order, not a hand-written sequence of system calls. See
[threading-model.md](threading-model.md).

## Current state

Every crate has a real implementation now, including every `*-driver`
(`audio-driver` is real, backed by `cpal` — see
[ADR 012](adr/012-audio-output-via-cpal.md); `platform-core::Window`/
`DynamicLibrary` are real too, `Window` via `winit` — see
[ADR 010](adr/010-windowing-via-winit.md) — `DynamicLibrary` via
hand-written `dlopen`/`LoadLibrary` FFI). `graphics-driver` has a real
*windowed* `wgpu` `Device` (`Device::new_windowed`, a real swapchain
`Surface`), and `graphics-core`'s scene/material/lighting/submission
bridge (`Scene3D`, `SceneRenderer`, real Blinn-Phong lit draws through
it) is real too — proven end-to-end by the `magic_figures`/
`physic_figures` examples.

`meridian-engine-core`'s `Runtime`/`SubsystemManager`/`EventSystem` are
real and tested in isolation (`Runtime::tick` advances physics then
recomputes audio from the result each frame, `EventSystem` is the
type-erased pub/sub mechanism rule 7 exists for), but — unlike every
other crate named above — **no example actually composes `Runtime` yet**:
`magic_figures`/`physic_figures` each hand-roll their own physics/audio
wiring directly against `physics-core`/`audio-core` instead of going
through `SubsystemManager`, and `graphics-core` was never wired into
`Runtime::tick` at all (not because it lacks a scene vocabulary — it
doesn't, see above — but because presenting a frame needs a real
windowed `Device`/`Surface`, driver state `engine-core` deliberately
never depends on). See [roadmap.md](roadmap.md) for the exact state of
each crate and the `Runtime`-adoption follow-up.
