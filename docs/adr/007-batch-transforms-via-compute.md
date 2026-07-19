# ADR 007: Batch transforms execute through compute-core, not gac-core

## Status

Accepted

## Context

`gac-core`'s `Motor3` is the engine-wide `Transform` (see
[ADR 001](001-geometric-algebra-as-spatial-model.md)). Individual transform
math (composing a parent/child motor, transforming a point) is cheap enough
that scalar CPU code is the right default. But subsystems that touch large,
mostly-independent batches of transforms in one pass — `physics-core`'s
broad phase, `graphics-core`'s culling and scene extraction, a future
`animation-core` skinning a crowd — need those batches to run on CPU-SIMD or
GPU compute at scale, per [dependency-rules.md](../dependency-rules.md)
rule 5.

GPU dispatch has fixed per-call latency (buffer upload, dispatch, sync) that
scalar CPU code does not. For a few hundred transforms, CPU wins outright.
For tens of thousands of independent transforms, GPU throughput wins despite
that latency. Neither `gac-core` nor any single subsystem crate should have
to encode that crossover, and `gac-core` in particular must stay ignorant of
compute entirely — rule 6 forbids it from depending on SIMD dispatch or CPU
feature detection, let alone a GPU backend.

## Decision

`meridian-compute-core` depends on `meridian-gac-core` (a new edge, rule
10) and defines `TransformBatchKernel`: a `ComputeTask` specialized to a
batch of `Motor3`s. `ComputeScheduler` picks CPU or GPU execution per batch
using `GPU_DISPATCH_THRESHOLD`. `gac-core` depends on neither `compute-core`
nor `compute-driver` — the edge only ever points
`compute-core -> gac-core`, never back. Subsystems that need batched
transforms (`physics-core`, `graphics-core`, and eventually
`animation-core`/`particles-core`/`ai-core`) go through
`compute-core::TransformBatchKernel`, never through `compute-driver`
directly and never by re-deriving their own batch scheduling.

```text
        Transform API (Motor3, Rotor, Frame)  -- meridian-gac-core
                          |
              +-----------+-----------+
              |                       |
           CPU path              compute-core
       scalar/SIMD          TransformBatchKernel
              |                       |
          gameplay,             physics-core,
       small batches           graphics-core,
                               large batches
```

## Alternatives considered

- **Put batch/SIMD transform kernels directly in `gac-core`** — rejected:
  this is exactly what rule 6 exists to prevent. `gac-core` would have to
  know about `compute-driver`'s buffer/dispatch model, and every future
  compute consumer would pull that weight in through `gac-core` even if it
  never uses batching.
- **Let `physics-core`/`graphics-core` depend on `compute-driver` directly
  and build their own `Motor3` batch dispatch** — rejected: duplicates
  scheduling logic per subsystem and violates rule 5, which exists so
  `compute-core` is the single place that owns the CPU/GPU dispatch
  decision.
- **Always dispatch batch transforms to GPU compute** — rejected: for
  small batches (the common case in gameplay code — tens to low hundreds of
  transforms) the upload/dispatch/sync round-trip costs more than scalar CPU
  execution. `GPU_DISPATCH_THRESHOLD` exists so small batches don't pay GPU
  latency for no benefit.

## Consequences

- `compute-core` is no longer dependency-free within the workspace's
  spatial-math story: it now sits above `gac-core`, same as `ecs-core`,
  `physics-core`, `graphics-core`, and `audio-core` do. `gac-core` remains
  the lowest domain-logic layer above `numeric-core`/`foundation` and never
  gains a compute dependency in either direction.
- Any future compute-consuming crate (`animation-core`, `particles-core`,
  `ai-core`) that needs batch `Motor3` work reuses
  `TransformBatchKernel` instead of defining its own batch-transform type.
- The CPU/GPU crossover point (`GPU_DISPATCH_THRESHOLD`) is a single tunable
  constant in `compute-core`, not duplicated per subsystem.
