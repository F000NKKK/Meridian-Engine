# meridian-gac

Geometric Algebra Core — vectors, bivectors, multivectors, rotors and motors; the shared spatial math foundation for every other subsystem.

**Layer and context:** shared kernel · geometry

- **For:** Vectors, bivectors, multivectors, rotors and motors (`Vec3`, `Rotor`, `Motor3`) in float and `Fixed` flavors.
- The `GaFlavor` traits that physics and graphics are generic over.
- **Not for:** GPU or SIMD dispatch: `meridian-gac-wgpu` adapts these types to `meridian-compute-app`.

## Usage

```rust
use meridian_gac::{Rotor, Vec3};

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

- Depends on: [`meridian-numeric`](../meridian-numeric)
- Used by: [`meridian-audio-domain`](../meridian-audio-domain), [`meridian-ecs`](../meridian-ecs), [`meridian-gac-wgpu`](../meridian-gac-wgpu), [`meridian-rendering-domain`](../meridian-rendering-domain), [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-physics-app`](../meridian-physics-app), [`meridian-physics-wgpu`](../meridian-physics-wgpu), [`meridian-physics-domain`](../meridian-physics-domain), [`meridian-data-substrate`](../meridian-data-substrate), [`meridian-sdk`](../meridian-sdk), [`meridian-ui-scene2d`](../meridian-ui-scene2d)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
