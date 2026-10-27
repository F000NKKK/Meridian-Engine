# meridian-ecs-core

Archetype-based, data-oriented Entity Component System with SoA storage, built for cache- and SIMD-friendly iteration.

**Layer and context:** shared kernel · entity

- **For:** Entities, components, archetypes with SoA storage, and `Query`/`QueryMut`.
- The `Transform` component, a `Motor3`.
- **Not for:** graphics, physics or audio concepts: those stay in their own `*-core` crates.

## Usage

```rust
use meridian_ecs_core::{Component, World};

struct Position(f32);
impl Component for Position {}

let mut world = World::new();
let entity = world.spawn();
world.insert(entity, Position(1.0));
assert_eq!(world.get::<Position>(entity).map(|p| p.0), Some(1.0));
assert_eq!(world.query::<Position>().count(), 1);
```

## Depends on / used by

- Depends on: [`meridian-gac-core`](../meridian-gac-core), [`meridian-memory-core`](../meridian-memory-core)
- Used by: [`meridian-editor-protocol`](../meridian-editor-protocol), [`meridian-engine-core`](../meridian-engine-core), [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-physics-core`](../meridian-physics-core), [`meridian-scene`](../meridian-scene)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
