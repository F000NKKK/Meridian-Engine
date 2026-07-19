# GAC design — `meridian-gac-core`

The Geometric Algebra Core is the spatial-math foundation every other
subsystem builds on. It plays the role glm/Eigen/DirectXMath play in other
engines, but is built around geometric algebra (specifically PGA — projective
geometric algebra — for 3D rigid transforms) instead of raw matrices.

## `float_ga` and `fixed_ga`

Two GA modules, both re-exported: `float_ga` (`f32`, `Scalar` — the
default; re-exported at the crate root, so `meridian_gac_core::Vec3`/
`Motor3`/etc. resolve to it unchanged) and `fixed_ga` (`Fixed`, Q16.16
fixed-point from `meridian-numeric-core` — deterministic, opt-in). Each is
**self-contained**: algebra (`Multivector`/`Vec3`/`Bivector3`/`Rotor`/
`Motor3`) *and* geometric primitives (`Aabb`/`Sphere`/`Obb`/`Cone`/
`Plane`/`Shape`/`ConvexVolume`) both exist in both flavors
(`FixedAabb`/`FixedSphere`/`FixedObb`/`FixedCone`/`FixedPlane`/
`FixedShape`/`FixedConvexVolume` in `fixed_ga`). Neither flavor is
`physics-core`-local or owned by whichever crate needed it first — a
`Fixed`-based CPU-deterministic simulation for *any* subsystem
(`graphics-core`, a large precise CPU/GPU-emulated simulation, not just
`physics-core::deterministic`) can reach for `fixed_ga`'s primitives
directly, the same way any subsystem already reaches for `float_ga`'s.
`Projection` has no `Fixed` equivalent — it feeds a GPU pipeline directly,
which is `f32`-only regardless of which scalar type produced the camera's
pose (see [ADR 008](adr/008-fixed-point-determinism.md)).

`fixed_ga` is a disclosed, one-for-one duplication of `float_ga`'s
structure, not a generic `Motor3<S>`/`Shape<S>` — see
[ADR 008](adr/008-fixed-point-determinism.md) for why: `gac-compute`'s GPU
dispatch path has no good answer for fixed-point regardless (GPUs are
`f32`-native, no real `i64`), so the duplication a generic type would
avoid still has to exist at the GPU-relevant instantiation site. The pure
integer bitmask `blade` module (shared by both `Multivector` types — it
doesn't depend on the scalar type at all) lives once at the crate root,
not duplicated.

### Cross-flavor interop

Both flavors implement the same method names (`dot`, `cross`, `length`,
`normalize`, `compose`, `transform_point`, ...) by construction — `Fixed`
implements the same arithmetic operators `f32` does (see
`meridian-numeric-core`), so `fixed_ga` mirrors `float_ga` call-for-call.
Converting between them is a **named method, not `From`/`Into`**:
`Vec3::to_fixed_lossy()` / `FixedVec3::to_float_lossy()` (and the same
pair for `Bivector3`, `Rotor`, `Motor3`). `From`/`Into` would make a
precision- and determinism-changing cast look free at the call site
(`.into()` gives no hint anything happened); a `_lossy`-suffixed method
says so out loud everywhere it's used. `Vec3`/`FixedVec3` also support
direct mixed-type `+`/`-`/`*` (e.g. `vec3 + fixed_vec3`) built on the same
named conversions internally — useful when a value computed
deterministically on CPU needs combining with `f32` data headed for the
GPU, without hand-writing the conversion at every call site.

## Core types

```rust
Multivector   // the general element of the algebra
Blade         // a single k-vector term (scalar, vector, bivector, ...)
Rotor         // pure rotation
Motor         // rotation + translation, composable
Frame         // a named reference frame (origin + basis)
Projection    // camera/projective mappings (float_ga only — GPU-bound)
Aabb          // axis-aligned bounding box
Sphere        // center + radius
Obb           // oriented (rotated) box
Cone          // apex + axis + half-angle + height
Plane         // a half-space, normal . p + d >= 0
ConvexVolume  // an intersection of planes — a generalized frustum
Shape         // trait: any convex shape describable by a support function
```

`Aabb`, `Sphere`, `Obb`, `Cone` and `Plane` are plain geometric primitives
with no domain meaning of their own — `physics-core`'s broad phase and
`graphics-core`'s frustum culling both need a bounding box, `physics-core`'s
collider shapes are a sphere and an oriented box, and a plane has no reason
to be defined twice either. They live here rather than being redefined per
subsystem, for the same reason `Vec3`/`Motor3` do (see "Consumers" below).
`Obb`/`Aabb` are the two box variants — axis-aligned vs. oriented; a cube is
either one with equal extents on every axis, not a separate type. `Obb`
stores a `frame: Motor3` (not a separate center/orientation pair) — the
same pose convention every rigid body in the workspace uses (`RigidBody`,
`Camera`, `Listener`/`Emitter`), so a physics `RigidBody`'s own `frame` can
build an `Obb` directly, with no second orientation to keep in sync.

### `Shape`: one interface instead of a shape x shape matrix

Testing every shape pair (sphere vs. AABB, cone vs. OBB, ...) by hand is
combinatorial. Instead, every shape here implements one method — its
[support function](https://en.wikipedia.org/wiki/Support_function)
(`Shape::support(direction) -> Vec3`, the shape's own point farthest along
`direction`, the same interface GJK/EPA-style algorithms use) — and
`Plane::contains`/`ConvexVolume::intersects` are written once, generically,
against that trait. A new shape (a capsule, a convex hull) only needs to
implement `support`; every existing plane/volume test then works for it for
free, and no existing code has to learn about the new shape.

`ConvexVolume` generalizes `graphics-core`'s camera frustum (always exactly
six planes) to any number of planes, so it can describe any convex bounding
region, not just a camera's view volume — see docs/graphics-design.md for
how `Frustum` wraps it.

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

`Motor3` has three ways to apply itself: `transform_point` (the sandwich
product, points move by rotation *and* translation), `transform_vector`
(directions — computed exactly as `transform_point(v) -
transform_point(ZERO)`, which algebraically cancels the translation term,
not an approximation), and `to_mat4` (the classical matrix form graphics
APIs need — see docs/graphics-design.md). `Obb::support` uses
`transform_vector` to rotate a query direction into the box's local space.

## What lives outside `gac-core`

Scalar types, SIMD dispatch and CPU feature detection are **not** here —
they live in `meridian-numeric-core` (see
[dependency-rules.md](dependency-rules.md) rule 6). `gac-core` consumes
`numeric-core`'s scalar/SIMD abstractions; it does not define its own.

## Consumers

`ecs-core` (as the `Transform` component), `physics-core` (rigid body
frames), `graphics-core` (camera/object transforms), `audio-core` (listener/
emitter frames), `gac-compute` (batch `Motor3` kernels — see below). None of
them re-derive spatial math independently — that duplication is exactly what
this crate exists to prevent.

## Batch execution via gac-compute, not gac-core

`gac-core` defines what a `Motor3` *is*; it has no opinion about how a
million of them get transformed per frame, and it must never depend on
`meridian-compute-runtime` or `meridian-compute-driver` to find out (rule 6).
Symmetrically, `compute-runtime` must never depend on `gac-core` — it is a
generic CPU-SIMD/GPU dispatch runtime with no idea what a `Motor3` is (rule
5). Neither crate should have to compromise that purity just to make batch
transforms possible, so the batch path lives in a third crate,
`meridian-gac-compute`, which depends on both (rule 10; see
[ADR 007](adr/007-batch-transforms-via-compute.md)):

```text
                 Transform API (Motor3, Rotor, Frame)
                      |
              meridian-gac-core          meridian-compute-runtime
                      |                            |
                      +-------------+--------------+
                                    |
                          meridian-gac-compute
                        MotorTransformKernel
                        MotorComposeKernel
                                    |
                          GPU / CPU-SIMD dispatch
                             via compute-driver
                                    |
                         physics-core, graphics-core
                                (large batches)
```

The diagram above is the batch path only. Small transform counts (gameplay
code touching a handful of entities) skip `gac-compute` entirely and call
`gac-core`'s `Motor3` math directly (`motor * local`) — that's still the
cheapest path, and it never touches `compute-runtime`. `gac-compute`'s
kernels exist for the batch case: `physics-core`/`graphics-core` hand a
`Vec<Motor3>` to `MotorTransformKernel`, and it dispatches through
`compute-runtime` on whichever backend the scheduler picks. Either way the
math is the same `Motor3` geometric product — `gac-core` and
`compute-runtime` never have to know about each other for it to work.

See [ADR 001](adr/001-geometric-algebra-as-spatial-model.md) for why PGA was
chosen over quaternions + matrices.
