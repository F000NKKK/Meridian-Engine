---
name: meridian-data-oriented-ecs-keeper
description: Protect Meridian-Engine ECS storage boundaries. Use when editing meridian-ecs-core, components, archetypes, queries, World/Entity behavior, or docs about data-oriented design and ECS ownership.
---

# Meridian Data-Oriented ECS Keeper

## Workflow

1. Read `AGENTS.md`, `docs/ecs-design.md`, and `docs/adr/004-data-oriented-design.md`.
2. Preserve archetype/SoA storage and data-oriented iteration assumptions.
3. Keep ECS generic. Do not add graphics, physics, audio, asset, gameplay, material, rigid-body, DSP, or resource-lifetime semantics.
4. Treat `Transform` as the narrow shared spatial primitive built on `gac-core`, not as permission for domain leakage.
5. Be careful with type erasure, aliasing, entity migration, and stale/dead entity behavior.

## Validation

- Run `cargo test -p meridian-ecs-core`.
- Add focused tests for spawn/despawn, insert/remove migration, query/query_mut behavior, and dead/unknown entity access.
