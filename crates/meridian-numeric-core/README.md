# meridian-numeric-core

Scalar types, SIMD helpers, numeric traits and CPU feature dispatch underlying the GAC — kept separate so gac-core stays pure geometric algebra.

**Layer and context:** shared kernel · geometry

- **For:** The `Scalar` (`f32`) alias and the deterministic `Fixed` (Q16.16) type.
- Numeric traits and CPU feature dispatch used by the geometric algebra.
- **Not for:** geometric algebra: vectors, rotors and motors live in `meridian-gac-core`.

## Usage

```rust
use meridian_numeric_core::Fixed;

let a = Fixed::from_num(1.5);
let b = Fixed::from_num(2.0);
assert_eq!(a * b, Fixed::from_num(3.0));
```

## Features

| Feature | Enables |
|---|---|
| `data` | Substrate Data schema of `Fixed` (off by default) |

## Depends on / used by

- Depends on: [`meridian-foundation`](../meridian-foundation)
- Used by: [`meridian-gac-compute`](../meridian-gac-compute), [`meridian-gac-core`](../meridian-gac-core), [`meridian-physics-compute`](../meridian-physics-compute), [`meridian-physics-core`](../meridian-physics-core)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
