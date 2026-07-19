# ADR 001: Geometric algebra as the spatial model

## Status

Accepted

## Context

Classic engines represent spatial transforms differently per subsystem:
graphics uses 4x4 matrices, physics uses a position/quaternion pair, audio
often has its own listener-relative convention. Keeping these in sync is
manual, error-prone work, and each representation has its own edge cases
(gimbal lock for Euler angles, quaternion/position drift, matrix shear from
accumulated scale).

## Decision

Use projective geometric algebra (PGA) as the single spatial representation
shared by every subsystem. A `Motor3` (translation + rotation, composable
via the geometric product) replaces the separate position/rotation/scale
triple as the engine's `Transform`. See
[gac-design.md](../gac-design.md) for the concrete types.

## Alternatives considered

- **Matrices (glm/DirectXMath style)** — familiar, well-tooled, but
  transform composition and interpolation (slerp-equivalent) are more
  awkward, and scale bakes into the same object as rotation, inviting shear
  bugs.
- **Quaternion + vector pair** — avoids gimbal lock for rotation alone, but
  is two objects that must be kept in sync, and composing a full rigid
  transform (rotate then translate vs. translate then rotate) is easy to
  get backwards.

## Consequences

- Every subsystem that needs a transform (`ecs-core`'s `Transform`
  component, `physics-core`'s `RigidBody::frame`, `graphics-core`'s camera/
  object transforms, `audio-core`'s listener/emitter frames) uses the same
  `Motor3` from `gac-core` — no per-subsystem transform type, no conversion
  layer between them.
- The team takes on the cost of PGA being less familiar than matrices;
  mitigated by keeping `gac-core`'s public API concept-first (`Motor`,
  `Rotor`, `Frame`) rather than requiring callers to know the algebra.
- `gac-core` must stay dependency-light and correct before any subsystem
  above it can be meaningfully implemented — see the suggested
  implementation order in [roadmap.md](../roadmap.md).
