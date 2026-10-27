---
name: meridian-gac-math-keeper
description: Preserve Meridian-Engine geometric algebra semantics. Use when editing meridian-gac, Motor3/Rotor/Bivector3/Vec3 transform code, physics rotation/angular quantities, camera/listener transforms, or docs about the shared spatial model.
---

# Meridian GAC Math Keeper

## Workflow

1. Read `AGENTS.md`, `docs/gac-design.md`, and `docs/adr/001-geometric-algebra-as-spatial-model.md`.
2. Preserve the vector/bivector distinction. Angular velocity, torque, and rotation algebra live as bivectors when semantics require it.
3. Keep `gac` pure math: no GPU, compute-app, platform, scheduler, or target-feature dispatch decisions.
4. Put batched execution in `meridian-gac-wgpu`, not in `gac`.
5. Add tests against independent oracles where possible, such as Rodrigues rotation or direct scalar formulas, not only project self-consistency.
6. Float/fixed code is one directory per concept (`rotor/{mod,float,fixed}.rs`, ADR 020 D5); do not recreate `generic.rs`/`float_ga`/`fixed_ga`.

## Validation

- Run `cargo test -p meridian-gac` for GAC changes.
- Run affected downstream tests, especially `cargo test -p meridian-gac-wgpu` and `cargo test -p meridian-physics-domain`, when transform behavior changes.
