---
name: meridian-crate-boundary-keeper
description: Enforce Meridian-Engine crate boundaries and Cargo dependency direction. Use when editing any Cargo.toml, adding or moving public APIs across crates, adding a new crate, fixing dependency-related compile errors, or touching docs/dependency-rules.md.
---

# Meridian Crate Boundary Keeper

## Workflow

1. Read `AGENTS.md`, `docs/dependency-rules.md`, `docs/context-map.md` and ADR 018 before changing an edge; layers per crate are in `craftbook.toml`.
2. Inspect current edges with crate `Cargo.toml` files; use `cargo tree --workspace --edges normal` if needed.
3. Reject forbidden edges: anything pointing outwards (domain/app → driver or adapter, kernel → anything but kernel), a driver to another subsystem's driver or domain, `gac-core` ↔ `compute-runtime`, `asset-core` to ECS/graphics/engine, `resource-core` beyond memory-core.
4. Point dependencies inwards: the inner crate declares a port trait, the adapter crate implements it (ADR 018). Edges in `PENDING` of `scripts/check_dependency_rules.py` only shrink; never add one.
5. Keep cross-subsystem orchestration in `engine-core`.
6. Update docs/ADRs when adding a new permitted edge or crate role.

## Validation

- Always run `./build.sh check-deps` and `craftcheck deps . --baseline .craftcheck-deps-baseline` after dependency changes.
- Run `cargo test --workspace` after public API or graph changes.
