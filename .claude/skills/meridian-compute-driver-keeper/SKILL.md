---
name: meridian-compute-driver-keeper
description: Preserve Meridian compute dispatch boundaries. Use when editing meridian-compute-wgpu, meridian-compute-app, meridian-gac-wgpu, future concrete *-compute adapters, GPU/SIMD dispatch code, buffers, kernels, or compute/threading docs.
---

# Meridian Compute Driver Keeper

## Workflow

1. Read `AGENTS.md`, `docs/dependency-rules.md`, `docs/threading-model.md`, and `docs/adr/007-batch-transforms-via-compute.md`.
2. Keep `compute-app` generic: device/context, untyped buffers, dispatch ordering, and `ComputeKernel` mechanics only.
3. Do not put domain-shaped algorithms or buffers in `compute-app`; no `Motor3`, rigid bodies, particles, or domain buffer wrappers there.
4. Put GAC batch kernels in `meridian-gac-wgpu`; add future domain adapter crates only for concrete needs.
5. Do not reimplement general engine task scheduling inside compute runtime. Respect `runtime-threads` boundaries.
6. Keep low-level CPU/GPU execution in `compute-wgpu`, as an adapter of the compute device port `compute-app` owns (ADR 019); do not add new `compute-app → compute-wgpu` reach.

## Validation

- Run `cargo test -p meridian-compute-wgpu`, `cargo test -p meridian-compute-app`, and `cargo test -p meridian-gac-wgpu` as applicable.
- Test zero/small batches and large batches that exercise parallel paths.
- Run `./build.sh check-deps` after any edge change.
