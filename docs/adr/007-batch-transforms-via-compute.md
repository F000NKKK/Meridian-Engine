# ADR 007: Batch GAC transforms via a gac-core/compute-runtime adapter crate

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

That capability sits between two crates that must both stay narrow:
`gac-core` must stay pure geometric algebra and never learn that a GPU
exists (rule 6) — it needs to keep working unmodified in the editor,
serialization, networked replication, or any other context that has nothing
to do with compute. `compute-runtime` must stay a generic dispatch runtime
and never learn what a `Motor3` is — the same crate has to serve a future
`particle-compute`, `physics-compute`, or `ai-compute` without carrying GAC
baggage those consumers don't need. Making either crate depend on the other
to get batch transforms working would compromise exactly the property that
makes each of them reusable outside this one use case.

## Decision

A third crate, `meridian-gac-compute`, is the adapter: it depends on both
`meridian-gac-core` and `meridian-compute-runtime` and is the *only* crate
allowed to (rule 10). It defines the batch kernels —
`MotorTransformKernel`, `MotorComposeKernel` — that implement
`compute-runtime`'s `ComputeKernel` interface for `gac-core`'s `Motor3`.
`gac-core` and `compute-runtime` never depend on each other, in either
direction. Subsystems that need batched transforms (`physics-core`,
`graphics-core`, and eventually `animation-core`/`particles-core`/
`ai-core`) depend on `gac-compute` for that batch path, and on
`compute-runtime` directly for any non-GAC compute work (e.g. GPU culling)
they also need.

```text
meridian-gac-core        meridian-compute-runtime
        |                          |
        +------------+-------------+
                      |
            meridian-gac-compute
          MotorTransformKernel
          MotorComposeKernel
                      |
        +-------------+-------------+
        |                           |
   graphics-core               physics-core
```

The same shape generalizes: any future domain that needs batched compute
gets its own `<domain>-compute` adapter crate depending on its domain crate
plus `compute-runtime`, rather than either the domain crate or
`compute-runtime` absorbing the other's concerns.

## Alternatives considered

- **`compute-runtime` (then `compute-core`) depends on `gac-core`
  directly** — the first version of this decision. Rejected on review: it
  makes the generic compute runtime carry a permanent GAC dependency, so
  every future `<domain>-compute` consumer of `compute-runtime` pulls in
  `gac-core` whether it touches transforms or not, and `compute-runtime`
  stops being a clean, domain-agnostic dispatch layer.
- **Put batch/SIMD transform kernels directly in `gac-core`** — rejected:
  this is exactly what rule 6 exists to prevent. `gac-core` would have to
  know about `compute-driver`'s buffer/dispatch model, and `gac-core` would
  no longer be usable in compute-free contexts (editor tooling,
  serialization, network replication) without pulling in the whole compute
  stack.
- **Let `physics-core`/`graphics-core` depend on `compute-driver` directly
  and build their own `Motor3` batch dispatch** — rejected: duplicates
  scheduling logic per subsystem and violates rule 5, which exists so
  `compute-runtime` is the single place that owns CPU/GPU dispatch
  mechanics.

## Consequences

- `gac-core` stays reusable in every context that needs spatial math but
  not compute — exactly the set of future uses (editor, serialization,
  networking) that motivated this decision.
- `compute-runtime` stays a small, stable, domain-agnostic dispatch layer:
  `ComputeContext`, `ComputeKernel`, buffers, dispatch sizing — nothing that
  changes when a new domain adopts it.
- Every future batch-compute domain (`particle-compute`, `physics-compute`,
  `ai-compute`, ...) follows the same three-crate shape instead of growing
  a direct edge into `compute-runtime`'s internals or `gac-core`'s.
- One more crate in the workspace than the two-crate version of this
  decision — accepted as the cost of keeping `gac-core` and
  `compute-runtime` mutually independent.
