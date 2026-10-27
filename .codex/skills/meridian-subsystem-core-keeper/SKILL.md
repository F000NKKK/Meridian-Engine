---
name: meridian-subsystem-core-keeper
description: Maintain Meridian graphics, physics, audio core/driver separation. Use when editing meridian-graphics-core, meridian-physics-core, meridian-audio-core, matching driver crates, or subsystem design docs.
---

# Meridian Subsystem Core Keeper

## Workflow

1. Read `AGENTS.md`, `docs/dependency-rules.md`, ADR 018 (supersedes ADR 005), and the matching subsystem design doc.
2. Keep domain algorithms in `*-core`; keep hardware/OS execution abstraction in `*-driver`.
3. A core declares the ports it needs from hardware (traits in the domain or application crate); its driver depends on that crate and implements them. Never add an edge from a core to any driver; the existing core → driver edges are pending migration (wave 5).
4. A driver knows only the port's types: no scenes, materials, rigid bodies or mixers.
5. Use `compute-runtime` or an adapter crate for compute work; do not depend directly on `compute-driver` from subsystem cores.
6. Keep cross-subsystem coordination out of domain cores unless the allowed dependency graph already models it.

## Validation

- Run package tests for touched crates and `./build.sh check-deps` for dependency changes.
- Add domain invariant tests: collision/contact/solver behavior for physics, render graph/culling ordering for graphics, and spatial mixer/listener behavior for audio.
