---
name: meridian-resource-lifetime-keeper
description: Keep Meridian memory, resource identity, asset decoding, and lifetime policy separated. Use when editing meridian-memory, meridian-resource, meridian-asset-domain, handles, resource pools, dependency graphs, decoders, or docs/ADRs about managers and resources.
---

# Meridian Resource Lifetime Keeper

## Workflow

1. Read `AGENTS.md`, `docs/memory-model.md`, `docs/adr/002-handle-based-resources.md`, `docs/adr/003-no-global-managers.md`, and `docs/adr/006-resource-core-separation.md`.
2. Keep generic generational handles and pools in `memory`.
3. Keep typed resource identity, versions, and dependency topology in `resource`.
4. Keep file bytes -> CPU-side decoded data in `asset-domain`.
5. Do not add `AssetManager`, `ResourceManager`, `CacheManager`, singleton access, automatic loading/eviction/reload policy, or asset-domain APIs returning `ResourceId`/handles.
6. Let the application/engine layer decide ownership and lifetime policy.

## Validation

- Run package tests for touched crates.
- Test stale handles, generation reuse, dependency cycles/transitive dependency queries, and malformed asset inputs.
