# GAC design — `meridian-gac-core`

The Geometric Algebra Core is the spatial-math foundation every other
subsystem builds on. It plays the role glm/Eigen/DirectXMath play in other
engines, but is built around geometric algebra (specifically PGA — projective
geometric algebra — for 3D rigid transforms) instead of raw matrices.

## Core types

```rust
Multivector   // the general element of the algebra
Blade         // a single k-vector term (scalar, vector, bivector, ...)
Rotor         // pure rotation
Motor         // rotation + translation, composable
Frame         // a named reference frame (origin + basis)
Projection    // camera/projective mappings
```

## `Transform` is a `Motor3`

Instead of the classic:

```rust
struct Transform {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
}
```

Meridian uses:

```rust
struct Transform {
    motor: Motor3,
}
```

Rotation and translation compose through the same operator (geometric
product), so chaining parent/child transforms is one multiplication instead
of a position-plus-rotation merge. Non-uniform scale is deliberately not
folded into the motor — it's a separate, explicit factor where a subsystem
needs it (e.g. rendering), because scale doesn't behave like a rigid motion
and baking it into the spatial primitive is what causes shear bugs in
matrix-based engines.

## What lives outside `gac-core`

Scalar types, SIMD dispatch and CPU feature detection are **not** here —
they live in `meridian-numeric-core` (see
[dependency-rules.md](dependency-rules.md) rule 6). `gac-core` consumes
`numeric-core`'s scalar/SIMD abstractions; it does not define its own.

## Consumers

`ecs-core` (as the `Transform` component), `physics-core` (rigid body
frames), `graphics-core` (camera/object transforms), `audio-core` (listener/
emitter frames), `compute-core` (batch `Motor3` kernels — see below). None of
them re-derive spatial math independently — that duplication is exactly what
this crate exists to prevent.

## Batch execution via compute-core

`gac-core` defines what a `Motor3` *is*; it has no opinion about how a
million of them get transformed per frame, and it must never depend on
`compute-core` or `compute-driver` to find out (rule 6). That decision
— CPU scalar/SIMD for a handful of transforms, GPU compute once a batch is
large enough that upload/dispatch/sync latency stops dominating — belongs to
`compute-core`'s `TransformBatchKernel`, which depends on `gac-core` for the
`Motor3` type (the one edge rule 10 allows in this direction; see
[ADR 007](adr/007-batch-transforms-via-compute.md)):

```text
                 Transform API (Motor3, Rotor, Frame)
                      |
              meridian-gac-core
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

`ComputeScheduler` picks the path per task using
`GPU_DISPATCH_THRESHOLD` — below it, a batch runs on the CPU path directly
against `Motor3`; at or above it, `TransformBatchKernel` dispatches through
`compute-driver`. Either way the math is the same `Motor3` geometric product
— only the execution backend changes, and `gac-core` never has to know which
one is running.

See [ADR 001](adr/001-geometric-algebra-as-spatial-model.md) for why PGA was
chosen over quaternions + matrices.
