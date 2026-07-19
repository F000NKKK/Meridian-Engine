# ADR 003: No global managers

## Status

Accepted

## Context

A common engine pattern is a global singleton — `AssetManager::instance()`,
`Engine::get()`, a static `ResourceManager` — that every subsystem reaches
into. It's convenient to call from anywhere, but it hides ownership,
complicates testing (global mutable state), and makes lifetime and
threading reasoning implicit instead of explicit.

## Decision

No crate in the workspace defines a global/singleton manager type.
Concretely: `meridian-asset-core` must never contain an `AssetManager`,
`ResourceManager`, or `CacheManager` (see
[dependency-rules.md](../dependency-rules.md) rule 4); resource pools live
in `meridian-memory-core` as values the application constructs and owns,
not as statics any crate can reach into unprompted.

## Alternatives considered

- **Global singleton managers** — the common pattern, rejected for the
  reasons above: implicit ownership, harder testing, and it invites
  exactly the kind of cross-layer coupling
  [dependency-rules.md](../dependency-rules.md) exists to prevent (once
  every subsystem can reach a global `AssetManager`, nothing stops
  `ecs-core` from calling into it directly).
- **Dependency injection framework** — over-engineered for this workspace's
  needs; explicit ownership passed through constructors/parameters is
  sufficient at this scale and keeps the dependency graph honest (a crate
  that doesn't receive a pool can't secretly reach one).

## Consequences

- The application layer is responsible for constructing resource pools,
  deciding their lifetime, and threading handles to the subsystems that
  need them. This is more boilerplate at the call site than a global
  singleton, and is the accepted cost.
- Every core crate's dependencies are visible in its `Cargo.toml` — there's
  no hidden global it can reach that isn't declared. This keeps
  [dependency-rules.md](../dependency-rules.md) actually enforceable:
  static analysis of the dependency graph is meaningful only if nothing
  routes around it through a singleton.
- Testing any crate in isolation doesn't require resetting global state
  between tests.
