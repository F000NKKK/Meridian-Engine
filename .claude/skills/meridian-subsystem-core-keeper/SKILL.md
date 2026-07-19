---
name: meridian-subsystem-core-keeper
description: Maintain Meridian graphics, physics, audio core/driver separation. Use when editing meridian-graphics-core, meridian-physics-core, meridian-audio-core, matching driver crates, or subsystem design docs.
---

# Meridian Subsystem Core Keeper

## Workflow

1. Read `AGENTS.md`, `docs/dependency-rules.md`, ADR 005, and the matching subsystem design doc.
2. Keep domain algorithms in `*-core`; keep hardware/OS execution abstraction in `*-driver`.
3. A core may depend on its matching driver only. Never add a dependency on another subsystem's driver.
4. A driver must not depend on its own core.
5. Use `compute-runtime` or an adapter crate for compute work; do not depend directly on `compute-driver` from subsystem cores.
6. Keep cross-subsystem coordination out of domain cores unless the allowed dependency graph already models it.

## Validation

- Run package tests for touched crates and `./build.sh check-deps` for dependency changes.
- Add domain invariant tests: collision/contact/solver behavior for physics, render graph/culling ordering for graphics, and spatial mixer/listener behavior for audio.
