# meridian-ecs

Archetype-based, data-oriented Entity Component System with `SoA` storage, built for cache- and SIMD-friendly iteration.

**Layer and context:** shared kernel · entity

- **For:** Entities, components, archetypes with `SoA` storage, and `Query`/`QueryMut` and `query_many`/`query_many_mut` over two to four components, with `Option` elements and `With`/`Without` filters; `World::component_types(entity)` lists an entity's components; `Changed<T>` with `World::query_many_since` skips unchanged archetype columns; `Commands` defer structural changes made during iteration.
- The `Transform` component, a `Motor3`.
- **Not for:** graphics, physics or audio concepts: those stay in their own `*-core` crates.

## Usage

```rust
use meridian_ecs::{Component, World};

struct Position(f32);
impl Component for Position {}

let mut world = World::new();
let entity = world.spawn();
world.insert(entity, Position(1.0));
assert_eq!(world.get::<Position>(entity).map(|p| p.0), Some(1.0));
assert_eq!(world.query::<Position>().count(), 1);
```

## Depends on / used by

- Depends on: [`meridian-gac`](../meridian-gac), [`meridian-memory`](../meridian-memory)
- Used by: [`meridian-editor-app`](../meridian-editor-app), [`meridian-editor-domain`](../meridian-editor-domain), [`meridian-rendering-domain`](../meridian-rendering-domain), [`meridian-data-substrate`](../meridian-data-substrate)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
