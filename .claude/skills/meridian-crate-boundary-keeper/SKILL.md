---
name: meridian-crate-boundary-keeper
description: Enforce Meridian-Engine crate boundaries and Cargo dependency direction. Use when editing any Cargo.toml, adding or moving public APIs across crates, adding a new crate, fixing dependency-related compile errors, or touching docs/dependency-rules.md.
---

# Meridian Crate Boundary Keeper

## Workflow

1. Read `AGENTS.md` and `docs/dependency-rules.md` before changing an edge.
2. Inspect current edges with crate `Cargo.toml` files; use `cargo tree --workspace --edges normal` if needed.
3. Reject forbidden edges: driver-to-own-core, core-to-other-subsystem-driver, `gac-core` ↔ `compute-runtime`, `asset-core` to ECS/graphics/engine, `resource-core` beyond memory-core.
4. Prefer an adapter crate over reversing a dependency direction.
5. Keep cross-subsystem orchestration in `engine-core`.
6. Update docs/ADRs when adding a new permitted edge or crate role.

## Validation

- Always run `./build.sh check-deps` after dependency changes.
- Run `cargo test --workspace` after public API or graph changes.
