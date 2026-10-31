# Spatial recipes for people and coding agents

Status: checked float constructors, look rotation, normalization and interpolation.
The CLI pipeline remains separate work.

Use `meridian_sdk::{Rotor, Motor3, Vec3, RotationError}` in game code.
The shared math crate exposes the same types through `meridian_gac`.
Do not construct raw multivector coefficients for ordinary game transforms.

## Conventions

- Axes X/Y/Z form a right-handed basis: X cross Y = Z. Positive rotation
  around Z maps X towards Y. Angles are **radians**, not degrees.
- Geometric vectors alone do not prescribe a model's forward axis. Declare it
  explicitly in game code; imported mesh geometry retains its authored axes.
- `a.compose(b)` applies **a first, b second**. It stores the product b * a.
- `Motor3::from_rotation_translation(r, t)` rotates first, then translates.
- `transform_point` includes translation; `transform_vector` does not.
- `reverse` undoes a **unit** rotor. Public raw coefficients can violate this
  precondition; arbitrary raw rotors are not validated by existing methods.

## Checked construction and aiming

```rust
use meridian_sdk::{Rotor, RotationError, Vec3};
fn aim() -> Result<Rotor, RotationError> {
    let model_forward = Vec3::X;
    let target_direction = Vec3::new(1.0, 2.0, 3.0);
    Rotor::between_directions(model_forward, target_direction)
}
```

`Rotor::try_from_axis_angle(axis, radians)` rejects zero axes and non-finite
inputs. Finite nonzero axes are normalized without overflowing their length.
`between_directions(from, to)` rejects invalid directions, returns identity for
parallel directions and chooses a deterministic perpendicular axis for exact
opposites. Alignment alone does not constrain roll; it is not `look_rotation`.
Existing `from_axis_angle` remains unchecked for compatibility.

## Local and world movement

```rust
use meridian_sdk::{Motor3, Rotor, Vec3};
let turn = Rotor::from_axis_angle(Vec3::Z, std::f32::consts::FRAC_PI_2);
let pose = Motor3::from_rotation_translation(turn, Vec3::X);
let local_step = Motor3::translation(Vec3::X).compose(pose);
let world_step = pose.compose(Motor3::translation(Vec3::X));
// local_step sends the origin to (1, 1, 0); world_step to (2, 0, 0).
```

For rigid parent/child placement, `child_local.compose(parent_world)` applies
child coordinates first and the parent pose second. Scale is a separate concern;
use the engine transform hierarchy when scale is present.

Run the executable recipes with:
`cargo test -p meridian-gac --test rotation_recipes`.

## Aiming with controlled roll

```rust
use meridian_sdk::{Rotor, RotationError, Vec3};
fn aim_at(position: Vec3, target: Vec3) -> Result<Rotor, RotationError> {
    // This model's local forward is +Z and up is +Y.
    Rotor::look_rotation(target - position, Vec3::Y)
}
let rotor = aim_at(Vec3::ZERO, Vec3::X)?;
assert!((rotor.transform_vector(Vec3::Z) - Vec3::X).length() < 1e-5);
# Ok::<(), RotationError>(())
```

`look_rotation` rejects a normalized forward/up cross-product length <= 1e-6.
Choose another up hint near a pole. A camera viewing along local -Z needs an
explicit convention adjustment; do not silently reuse the +Z model recipe.

## Raw input and interpolation

Use `raw.try_normalized()?` after reading untrusted rotor coefficients. It rejects
non-finite values, zero norm and any non-rotation blades. Existing `reverse`,
`transform_vector` and `slerp` assume unit rotors; they remain unchecked.
`a.try_slerp(b, t)?` validates and normalizes both inputs and accepts finite `t`
only in [0, 1]. Extrapolation requires the existing unchecked `slerp` API.

## Orbit position and a rigid child pose

```rust
use meridian_sdk::{Motor3, Rotor, RotationError, Vec3};
let target = Vec3::new(10.0, 0.0, 0.0);
let orbit = Rotor::try_from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)?;
let eye = target + orbit.transform_vector(Vec3::Z * 5.0);
assert!((eye - Vec3::new(15.0, 0.0, 0.0)).length() < 1e-5);
// This is an orbit position, not a camera view matrix.
let parent = Motor3::from_rotation_translation(orbit, target);
let child_local = Motor3::translation(Vec3::Z * 2.0);
let child_world = child_local.compose(parent);
assert!((child_world.transform_point(Vec3::ZERO) - Vec3::new(12.0, 0.0, 0.0)).length() < 1e-5);
# Ok::<(), RotationError>(())
```

## Checklist for a coding agent

1. Declare the model's forward/up axes; do not infer them from its name.
2. Convert degrees using `angle.to_radians()` before constructing a rotation.
3. Use checked constructors at user/data input boundaries and propagate `RotationError`.
4. Apply `a.compose(b)` in application order. Do not substitute quaternion multiplication order.
5. Use point transforms for positions and vector transforms for offsets/directions.
6. Keep scale separate from rigid Motor3 examples; use the engine hierarchy for scaled parents.
7. Check the transformed basis vectors in a small test before wiring animations or controls.

Available checked signatures (float flavor):

```text
Rotor::try_from_axis_angle(axis: Vec3, radians: f32) -> Result<Rotor, RotationError>
Rotor::between_directions(from: Vec3, to: Vec3) -> Result<Rotor, RotationError>
Rotor::look_rotation(forward: Vec3, up: Vec3) -> Result<Rotor, RotationError>
rotor.try_normalized() -> Result<Rotor, RotationError>
a.try_slerp(b: Rotor, fraction: f32) -> Result<Rotor, RotationError>
```

These additions are not yet a published SDK version. Fixed-point rotors retain their
existing API; these checked float operations must not be assumed available on FixedRotor.

## Independent spatial components

```rust
use meridian_sdk::{Position, Rotor, Vec3, resolve_spatial_components};
use meridian_sdk::persistence::{Transform, World};
let mut world = World::new();
let entity = world.spawn();
world.insert(entity, Position(Vec3::new(1.0, 2.0, 3.0)));
world.insert(entity, Rotor::try_from_axis_angle(Vec3::Y, 0.5)?);
let issues = resolve_spatial_components(&mut world);
assert_eq!(issues, Vec::new());
assert!(world.get::<Transform>(entity).is_some());
# Ok::<(), meridian_sdk::RotationError>(())
```

Alternatively attach Motor3 as the complete local pose. Do not attach Motor3
alongside Position/Rotors to the same entity. SpatialStage resolves those sources
before TransformStage; diagnostics are available through a shared `issues()` handle.
Other components remain attached independently. Physics-body synchronization is
not supplied by this authoring stage; do not let it overwrite a dynamic body's pose.
