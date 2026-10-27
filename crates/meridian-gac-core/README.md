# meridian-gac-core

Geometric Algebra Core — vectors, bivectors, multivectors, rotors and motors; the shared spatial math foundation for every other subsystem.

**Layer and context:** shared kernel · geometry

- **For:** Vectors, bivectors, multivectors, rotors and motors (`Vec3`, `Rotor`, `Motor3`) in float and `Fixed` flavors.
- The `GaFlavor` traits that physics and graphics are generic over.
- **Not for:** GPU or SIMD dispatch: `meridian-gac-compute` adapts these types to `meridian-compute-runtime`.

## Usage

```rust
use meridian_gac_core::{Rotor, Vec3};

// Rotating X about Z by a quarter turn gives Y.
let rotor = Rotor::from_axis_angle(Vec3::Z, std::f32::consts::FRAC_PI_2);
let rotated = rotor.transform_vector(Vec3::X);
assert!((rotated - Vec3::Y).length() < 1e-5);
```

## Features

| Feature | Enables |
|---|---|
| `data` | Substrate Data schemas for the GA types (off by default) |

## Depends on / used by

- Depends on: [`meridian-numeric-core`](../meridian-numeric-core)
- Used by: [`meridian-audio-core`](../meridian-audio-core), [`meridian-ecs-core`](../meridian-ecs-core), [`meridian-gac-compute`](../meridian-gac-compute), [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-physics-app`](../meridian-physics-app), [`meridian-physics-compute`](../meridian-physics-compute), [`meridian-physics-core`](../meridian-physics-core), [`meridian-scene`](../meridian-scene), [`meridian-sdk`](../meridian-sdk), [`meridian-ui-render`](../meridian-ui-render)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
