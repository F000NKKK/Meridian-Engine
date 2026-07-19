---
name: meridian-gac-math-keeper
description: Preserve Meridian-Engine geometric algebra semantics. Use when editing meridian-gac-core, Motor3/Rotor/Bivector3/Vec3 transform code, physics rotation/angular quantities, camera/listener transforms, or docs about the shared spatial model.
---

# Meridian GAC Math Keeper

## Workflow

1. Read `AGENTS.md`, `docs/gac-design.md`, and `docs/adr/001-geometric-algebra-as-spatial-model.md`.
2. Preserve the vector/bivector distinction. Angular velocity, torque, and rotation algebra live as bivectors when semantics require it.
3. Keep `gac-core` pure math: no GPU, compute-runtime, platform, scheduler, or target-feature dispatch decisions.
4. Put batched execution in `meridian-gac-compute`, not in `gac-core`.
5. Add tests against independent oracles where possible, such as Rodrigues rotation or direct scalar formulas, not only project self-consistency.

## Validation

- Run `cargo test -p meridian-gac-core` for GAC changes.
- Run affected downstream tests, especially `cargo test -p meridian-gac-compute` and `cargo test -p meridian-physics-core`, when transform behavior changes.
