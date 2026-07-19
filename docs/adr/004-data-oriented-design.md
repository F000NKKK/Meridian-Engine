# ADR 004: Data-oriented design (archetype ECS, SoA storage)

## Status

Accepted

## Context

`meridian-ecs-core` needs a storage strategy for entity component data.
The two mainstream options are OOP-style entity objects (each entity a
struct holding its components) and data-oriented ECS (components stored
separately from entities, entities as an index into that storage).

## Decision

Archetype-based ECS with Structure-of-Arrays storage per archetype (see
[ecs-design.md](../ecs-design.md)): all `Transform`s for entities sharing an
archetype are contiguous, all `Velocity`s are contiguous in a separate
array, and so on — not an array of `(Transform, Velocity, ...)` structs.

## Alternatives considered

- **OOP entity objects / Array-of-Structs** — simpler mental model, but
  iterating "all transforms" touches unrelated component data interleaved
  in memory, defeating cache prefetching and ruling out straightforward
  SIMD iteration over a single component type.
- **Sparse-set ECS** — faster arbitrary add/remove-component than archetype
  moves, but slower iteration (an extra indirection per access). Rejected
  because Meridian's frame loop iterates over thousands of components every
  frame, while archetype changes are comparatively rare — see the
  iteration-vs-mutation tradeoff discussion in
  [ecs-design.md](../ecs-design.md).

## Consequences

- Systems that operate on a single component type across many entities
  (the common case: "integrate all velocities", "cull all bounding
  volumes") get contiguous, SIMD-friendly access for free from the storage
  layout, with no per-system optimization needed.
- Adding or removing a component from an entity moves it to a different
  archetype's storage — more expensive than a sparse-set toggle. This is
  accepted because it's the rarer operation.
- This is also why `gac-core`'s `Transform` being a single `Motor3` (ADR
  001) matters for ECS performance specifically: one contiguous field per
  entity instead of a position/rotation/scale triple spread across three
  separate arrays.
