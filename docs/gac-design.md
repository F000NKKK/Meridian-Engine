# GAC design — `meridian-gac`

The Geometric Algebra Core is the spatial-math foundation every other
subsystem builds on. It plays the role glm/Eigen/DirectXMath play in other
engines, but is built around geometric algebra (specifically PGA — projective
geometric algebra — for 3D rigid transforms) instead of raw matrices.

## Layout: one directory per concept

`gac` has one directory per concept (`multivector/`, `vector/`,
`bivector/`, `rotor/`, `motor/`, `volume/`, `projection/`, `mesh/`,
`flavor/`, `blade/`), each with a wiring-only `mod.rs`, a `float.rs` and a
`fixed.rs`, plus item-named files for logic shared by both flavors
(ADR 020 D5). Everything public is re-exported at the crate root:
`meridian_gac::Vec3`/`Motor3`/`Aabb`/etc. are the `float` flavor
(`f32`, `Scalar` — the default, GPU-dispatchable), and the `Fixed*` names
(`FixedVec3`, `FixedMotor3`, `FixedAabb`, ...) are the deterministic `fixed`
flavor (`Fixed`, Q16.16 from `meridian-numeric`, opt-in).

The **algebra** — `Multivector`/`Vec3`/`Bivector3`/`Rotor`/`Motor3` — is
concretely duplicated between each concept's `float.rs` and `fixed.rs`. This
is deliberate and disclosed: see [ADR 008](adr/008-fixed-point-determinism.md)
for why a generic `Motor3<S>` wasn't used: `gac-wgpu`'s GPU dispatch path
has no good answer for fixed-point regardless (GPUs are `f32`-native, no real
`i64`), so the duplication a generic type would avoid still has to exist at
the GPU-relevant instantiation site. The pure integer bitmask `blade`
module and the geometric product's `basis_product` do not depend on the
scalar type, so each exists once.

Geometric **primitives** — `Aabb`/`Sphere`/`Obb`/`Cone`/`Plane`/`Shape`/
`ConvexVolume` (`volume/`), `Projection` (`projection/`), `Frame`
(`motor/`) and `Mesh`/`Face`/`icosphere` (`mesh/`) — have no such
constraint: an AABB overlap test or a projection-matrix derivation is the
same sequence of operations regardless of scalar type, so they're written
**once**, generic over `flavor::GaFlavor` (an associated-type bundle of
`ScalarLike`/`VectorLike`/`BivectorLike`/`RotorLike`/`MotorLike`, each a
thin trait naming the operations generic code needs, implemented by
forwarding to the flavor types' own inherent methods). Each concept's
`float.rs`/`fixed.rs` holds thin aliases over the generic definitions
(`volume::float::Aabb = volume::Aabb<FloatFlavor>`,
`volume::fixed::FixedAabb = volume::Aabb<FixedFlavor>`, ...), re-exported at
the crate root, so call sites don't need to think about the generic
parameter. Only code that wants to *be* generic over the flavor (like
`physics-domain`'s engine; see docs/physics-design.md) names
`meridian_gac::volume::Aabb` and friends. The generic definitions live
in their concept module, not at the crate root, because a locally-defined
item always shadows a same-named glob-imported item in Rust: `Aabb<F>` at
the root would shadow the `Aabb` alias re-exported from `volume::float`.
Neither flavor's primitives are `physics-domain`-local — a `Fixed`-based
CPU-deterministic simulation for *any* subsystem can reach for the `Fixed*`
types directly. `Projection` has no meaningfully different `Fixed`
behavior; most consumers only build the `FloatFlavor` one, since a GPU
pipeline needs `f32` uniforms regardless of which flavor computed the
camera's pose (via `to_float_lossy`).

### Cross-flavor interop

Both flavors implement the same method names (`dot`, `cross`, `length`,
`normalize`, `compose`, `transform_point`, ...) by construction — `Fixed`
implements the same arithmetic operators `f32` does (see
`meridian-numeric`), so each `fixed.rs` mirrors its `float.rs` call-for-call.
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
Projection    // camera/projective mappings (float only — GPU-bound)
Aabb          // axis-aligned bounding box
Sphere        // center + radius
Obb           // oriented (rotated) box
Cone          // apex + axis + half-angle + height
Plane         // a half-space, normal . p + d >= 0
ConvexVolume  // an intersection of planes — a generalized frustum
Shape         // trait: any convex shape describable by a support function
```

`Aabb`, `Sphere`, `Obb`, `Cone` and `Plane` are plain geometric primitives
with no domain meaning of their own — `physics-domain`'s broad phase and
`rendering-domain`'s frustum culling both need a bounding box, `physics-domain`'s
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

`ConvexVolume` generalizes `rendering-domain`'s camera frustum (always exactly
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

## What lives outside `gac`

Scalar types, SIMD dispatch and CPU feature detection are **not** here —
they live in `meridian-numeric` (see
[dependency-rules.md](dependency-rules.md) rule 6). `gac` consumes
`numeric`'s scalar/SIMD abstractions; it does not define its own.

## Consumers

`ecs` (as the `Transform` component), `physics-domain` (rigid body
frames), `rendering-domain` (camera/object transforms), `audio-domain` (listener/
emitter frames), `gac-wgpu` (batch `Motor3` kernels — see below). None of
them re-derive spatial math independently — that duplication is exactly what
this crate exists to prevent.

## Batch execution via gac-wgpu, not gac

`gac` defines what a `Motor3` *is*; it has no opinion about how a
million of them get transformed per frame, and it must never depend on
`meridian-compute-app` or `meridian-compute-wgpu` to find out (rule 6).
Symmetrically, `compute-app` must never depend on `gac` — it is a
generic CPU-SIMD/GPU dispatch runtime with no idea what a `Motor3` is (rule
5). Neither crate should have to compromise that purity just to make batch
transforms possible, so the batch path lives in a third crate,
`meridian-gac-wgpu`, which depends on both (rule 10; see
[ADR 007](adr/007-batch-transforms-via-compute.md)):

```text
                 Transform API (Motor3, Rotor, Frame)
                      |
              meridian-gac          meridian-compute-app
                      |                            |
                      +-------------+--------------+
                                    |
                          meridian-gac-wgpu
                        MotorTransformKernel
                        MotorComposeKernel
                                    |
                          GPU / CPU-SIMD dispatch
                             via compute-wgpu
                                    |
                         physics-domain, rendering-domain
                                (large batches)
```

The diagram above is the batch path only. Small transform counts (gameplay
code touching a handful of entities) skip `gac-wgpu` entirely and call
`gac`'s `Motor3` math directly (`motor * local`) — that's still the
cheapest path, and it never touches `compute-app`. `gac-wgpu`'s
kernels exist for the batch case: `physics-domain`/`rendering-domain` hand a
`Vec<Motor3>` to `MotorTransformKernel`, and it dispatches through
`compute-app` on whichever backend the scheduler picks. Either way the
math is the same `Motor3` geometric product — `gac` and
`compute-app` never have to know about each other for it to work.

See [ADR 001](adr/001-geometric-algebra-as-spatial-model.md) for why PGA was
chosen over quaternions + matrices.
