---
name: meridian-compute-driver-keeper
description: Preserve Meridian compute dispatch boundaries. Use when editing meridian-compute-driver, meridian-compute-runtime, meridian-gac-compute, future concrete *-compute adapters, GPU/SIMD dispatch code, buffers, kernels, or compute/threading docs.
---

# Meridian Compute Driver Keeper

## Workflow

1. Read `AGENTS.md`, `docs/dependency-rules.md`, `docs/threading-model.md`, and `docs/adr/007-batch-transforms-via-compute.md`.
2. Keep `compute-runtime` generic: device/context, untyped buffers, dispatch ordering, and `ComputeKernel` mechanics only.
3. Do not put domain-shaped algorithms or buffers in `compute-runtime`; no `Motor3`, rigid bodies, particles, or domain buffer wrappers there.
4. Put GAC batch kernels in `meridian-gac-compute`; add future domain adapter crates only for concrete needs.
5. Do not reimplement general engine task scheduling inside compute runtime. Respect `task-core` boundaries.
6. Keep low-level CPU/GPU execution abstraction in `compute-driver`.

## Validation

- Run `cargo test -p meridian-compute-driver`, `cargo test -p meridian-compute-runtime`, and `cargo test -p meridian-gac-compute` as applicable.
- Test zero/small batches and large batches that exercise parallel paths.
- Run `./build.sh check-deps` after any edge change.
