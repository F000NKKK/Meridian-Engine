---
name: meridian-engine-integration-keeper
description: Guide Meridian engine-core and example integration work. Use when editing meridian-engine-core, examples/**, frame scheduling, event/subsystem orchestration, or docs about top-level runtime integration.
---

# Meridian Engine Integration Keeper

## Workflow

1. Read `AGENTS.md`, `docs/architecture.md`, `docs/threading-model.md`, and `docs/roadmap.md`.
2. Put cross-subsystem orchestration in `engine-core`, not in ad hoc dependencies between domain crates.
3. Keep examples small and demonstrative. Examples may compose crates; core crates must remain layered.
4. Do not make lower crates depend upward to satisfy example convenience.
5. Update roadmap/status docs if an example demonstrates newly implemented behavior.

## Validation

- Run `cargo test -p meridian-engine-core` for engine changes.
- Run changed examples with `./build.sh run <example> -- <args>`.
- Run `cargo test --workspace` for broad integration changes.
