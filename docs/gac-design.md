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
emitter frames). None of them re-derive spatial math independently — that
duplication is exactly what this crate exists to prevent.

See [ADR 001](adr/001-geometric-algebra-as-spatial-model.md) for why PGA was
chosen over quaternions + matrices.
