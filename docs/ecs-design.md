# ECS design — `meridian-ecs`

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

### Snapshots

`World::snapshot(&Cloners)` copies every entity with its generation, every
component value and the archetype layout into a `WorldSnapshot`;
`World::restore(snapshot)` puts the world back. It is in memory and format-free
(the on-disk forms are `meridian-data-substrate`'s, through its own codecs), and
meant for editor play/stop, rollback and tests. A world is type-erased, so the
application registers the `Clone` component types in `Cloners`; a world holding
an unregistered type returns `SnapshotError::Unregistered` and copies nothing.
An `Entity` held across the round trip names the same entity afterwards, and one
spawned after the snapshot is dead after the restore. The clock never runs
backwards: a restore advances it once and stamps every column changed, so
`Changed<T>` readers see the rollback.

### Hierarchy

`Parent(Entity)` is a plain component: the entity another one hangs under.
`propagate_transforms(&mut World)` writes a `GlobalTransform` for every entity
with a `Transform`, `local.compose(parent_global)` down each chain (the order
of `gac-wgpu`'s batch kernels). An entity whose parent is despawned, has no
`Transform` or is on a cycle is a root, so a dangling or looping link never
panics and never depends on visit order. A `GlobalTransform` is rewritten only
when its value changes, so a pass that moves nothing leaves `Changed<GlobalTransform>`
empty (`Changed` is per archetype column, as above). The pass resolves the whole
world: ~0.2 us per entity when nothing moved, ~0.5 us on a first pass
(`cargo bench -p meridian-ecs`); skipping untouched subtrees waits for a profile
that needs it. Nothing calls it yet: `runtime-app` runs it as a stage once a
caller needs world-space motors.

## What this crate is not

It does not know about frames, rendering, physics, or the application main
loop. It must not depend on `runtime-app`, `rendering-domain`, `physics-domain`,
`audio-domain`, or `asset-domain` — see
[dependency-rules.md](dependency-rules.md) rule 3. Its only dependencies are
`memory` (for the underlying storage/handles) and `gac` (so that
`Transform` can be a first-class, GAC-native component type rather than each
subsystem defining its own).

## Why archetype ECS specifically

The alternative — sparse-set or bitset ECS — trades faster
add/remove-component for slower iteration. Meridian favors iteration speed:
frame-to-frame, most entities don't change archetype, but every system
iterates over thousands of them. Archetype moves are the rarer operation and
can afford to be the more expensive one.

## Ownership boundary

`ecs` never decides *when* an entity is destroyed or *what* an
`AssetHandle` stored in a component points to — that's the application's
call, consistent with the "no global managers" principle
([ADR 003](adr/003-no-global-managers.md)).
