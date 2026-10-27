# ECS design — `meridian-ecs-core`

Data-oriented, archetype-based ECS. Not a framework for game logic — a
storage and query mechanism, full stop.

## What this crate is

```text
Entity      opaque generational id
Component   plain data, no behavior
Archetype   the set of component types an entity has; entities with the
            same archetype share storage
Query       iterates matching archetypes' component columns
Storage     Structure-of-Arrays per archetype
```

Storage layout, conceptually:

```text
Archetype(Transform, Velocity):
    Transform: [T1][T2][T3][T4]
    Velocity:  [V1][V2][V3][V4]
```

rather than an array of `(Transform, Velocity)` structs. This is what makes
a `Query<Transform>`/`QueryMut<Velocity>` a tight, SIMD-friendly,
cache-friendly loop instead of a pointer-chasing walk.

**Multi-component queries are `World::query_many` / `query_many_mut`.**
They take a tuple of two to four `&T` / `&mut T` elements and yield
`(Entity, (&A, &mut B, ..))` for every entity that has all of them:

```rust
for (_, (position, velocity)) in world.query_many_mut::<(&mut Position, &Velocity)>() {
    position.0 += velocity.0;
}
```

No `unsafe`. An archetype keeps its columns in a `HashMap`, and
`HashMap::get_disjoint_mut` returns several `&mut` columns after checking
their keys differ, so the borrow checker is satisfied by the standard
library, not by the crate. A tuple that names one component twice
(`(&mut T, &T)`) would alias a column, so `query_many_mut` panics on it
before iterating; `query_many` (shared `&World`, `&T` elements only) allows
it. `Query`/`QueryMut` stay for the single-component case. Not built yet:
`Option<&T>` / `Without<T>` filters and parallel iteration.

## What this crate is not

It does not know about frames, rendering, physics, or the application main
loop. It must not depend on `engine-core`, `graphics-core`, `physics-core`,
`audio-core`, or `asset-core` — see
[dependency-rules.md](dependency-rules.md) rule 3. Its only dependencies are
`memory-core` (for the underlying storage/handles) and `gac-core` (so that
`Transform` can be a first-class, GAC-native component type rather than each
subsystem defining its own).

## Why archetype ECS specifically

The alternative — sparse-set or bitset ECS — trades faster
add/remove-component for slower iteration. Meridian favors iteration speed:
frame-to-frame, most entities don't change archetype, but every system
iterates over thousands of them. Archetype moves are the rarer operation and
can afford to be the more expensive one.

## Ownership boundary

`ecs-core` never decides *when* an entity is destroyed or *what* an
`AssetHandle` stored in a component points to — that's the application's
call, consistent with the "no global managers" principle
([ADR 003](adr/003-no-global-managers.md)).
