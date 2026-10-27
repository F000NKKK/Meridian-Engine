# meridian-numeric

Scalar types, SIMD helpers, numeric traits and CPU feature dispatch underlying the GAC — kept separate so gac stays pure geometric algebra.

**Layer and context:** shared kernel · geometry

- **For:** The `Scalar` (`f32`) alias and the deterministic `Fixed` (Q16.16) type.
- Numeric traits and CPU feature dispatch used by the geometric algebra.
- **Not for:** geometric algebra: vectors, rotors and motors live in `meridian-gac`.

## Usage

```rust
use meridian_numeric::Fixed;

let a = Fixed::from_num(1.5);
let b = Fixed::from_num(2.0);
assert_eq!(a * b, Fixed::from_num(3.0));
```

## Features

| Feature | Enables |
|---|---|
| `data` | Substrate Data schema of `Fixed` (off by default) |

## Depends on / used by

- Depends on: [`meridian-diagnostics`](../meridian-diagnostics)
- Used by: [`meridian-gac-wgpu`](../meridian-gac-wgpu), [`meridian-gac`](../meridian-gac), [`meridian-physics-wgpu`](../meridian-physics-wgpu), [`meridian-physics-domain`](../meridian-physics-domain)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
