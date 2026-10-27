# meridian-physics-core

Rigid body dynamics built on the GAC: broad/narrow phase collision, constraint solving and integration.

**Layer and context:** domain · physics

- **For:** Rigid bodies, colliders, broad/narrow phase, the constraint solver and the integrator, generic over `GaFlavor`.
- Soft bodies.
- **Not for:** execution backends and batching: `meridian-physics-compute` adapts these kernels to the compute runtime.

## Usage

```rust
use meridian_gac_core::Vec3;
use meridian_physics_core::{ColliderShape, Integrator, RigidBody};

let integrator = Integrator::new(Vec3::new(0.0, -10.0, 0.0));
let mut bodies = vec![RigidBody {
    mass: 1.0,
    shape: ColliderShape::Sphere { radius: 0.5 },
    ..Default::default()
}];
integrator.step(&mut bodies, 0.1);
assert!(bodies[0].velocity.y < 0.0);
```

## Depends on / used by

- Depends on: [`meridian-gac-core`](../meridian-gac-core), [`meridian-numeric-core`](../meridian-numeric-core), [`meridian-resource-core`](../meridian-resource-core)
- Used by: [`meridian-physics-app`](../meridian-physics-app), [`meridian-physics-compute`](../meridian-physics-compute), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
