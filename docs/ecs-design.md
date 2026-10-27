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

No `unsafe`. An archetype keeps its columns in a `HashMap`; one
`iter_mut` pass over it hands each accessed column to exactly one tuple
element, so the borrow checker is satisfied by the iterator, not by the
crate. A tuple that names one component twice
(`(&mut T, &T)`) would alias a column, so `query_many_mut` panics on it
before iterating; `query_many` (shared `&World`, `&T` elements only) allows
it. `Query`/`QueryMut` stay for the single-component case. Tuple elements may also be
`Option<&T>` / `Option<&mut T>` (the archetype need not have the component;
absent rows yield `None`) and the zero-sized filters `With<T>` / `Without<T>`,
which constrain the archetype without borrowing a column, so `With<T>` next
to `&mut T` is not an alias. Not built yet: parallel iteration.

### Deferred commands

A query holds the world borrowed, so a loop cannot spawn, despawn or change an
entity's component set. `Commands` is a plain value (no global, no hidden
queue in `World`): the loop records `spawn` / `despawn` / `insert` / `remove`
into it and the owner calls `commands.apply(&mut world)` after the loop.
Commands run in recorded order; one that targets a dead entity does nothing.
A deferred spawn hands a closure the new entity and the world, since the
entity id does not exist before `apply`.

### Change detection

The world has a clock (`World::tick`, `advance_tick`). Every column
remembers the tick it last changed at, and `Changed<T>` is a filter element
that keeps an archetype only if its `T` column changed at or after the `since`
tick given to `query_many_since` / `query_many_mut_since`. A system keeps the
value `advance_tick` returned when it last finished and passes it as `since`:
its own writes carry an earlier tick, so it does not see them back, and a write
by anything that ran after it is never missed (the comparison is "at or
after").

The granularity is the archetype column, not the row, on purpose: it is what a
renderer needs to skip whole unchanged instance batches, and it costs one
`Tick` per column. A write to one entity's `T` marks every entity of that
archetype's `T` column. Marked as changed: `insert` (overwrite or a new
archetype), `remove`, `despawn`, `get_mut`, `query_mut`, and the columns behind
`&mut` / `Option<&mut>` elements of `query_many_mut` as soon as they are handed
out, whether or not the loop writes. Shared queries never mark anything.

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
