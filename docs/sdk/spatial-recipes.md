# Spatial recipes for people and coding agents

Status: checked float rotation constructors and composition recipes. This is not
an assertion that look rotation, validated raw rotors or the CLI pipeline are complete.

Use `meridian_gac::{Rotor, Motor3, Vec3, RotationError}`. The SDK facade also exports
`Rotor`, `Motor3`, `Vec3`; errors can be propagated without naming their type.
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
use meridian_gac::{Rotor, RotationError, Vec3};
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
use meridian_gac::{Motor3, Rotor, Vec3};
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
