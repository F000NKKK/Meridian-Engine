# ADR 002: Handle-based resources instead of `Arc<T>`

## Status

Accepted

## Context

Textures, meshes, buffers and similar GPU/CPU resources need a way for
multiple subsystems to reference the same underlying data without owning
its lifetime individually. The obvious Rust default is `Arc<Texture>`.

## Decision

Resources are referenced by a `Copy`, plain-data generational handle:

```rust
struct Handle {
    index: u32,
    generation: u32,
}
```

held in a `meridian-memory-core` resource pool, not by `Arc<T>`. See
[memory-model.md](../memory-model.md).

## Alternatives considered

- **`Arc<T>`** — idiomatic Rust, zero extra machinery. Rejected because:
  reference counting couples lifetime across every subsystem holding a
  clone (a resource can't be reloaded/hot-swapped out from under existing
  `Arc`s without every holder cooperating), it's not trivially
  serializable, and atomic refcounting has a real cost at the scale of
  per-frame handle copies (thousands of components referencing textures/
  meshes).
- **Raw index into a `Vec`** — as fast as a handle, but a stale index after
  a slot is reused silently accesses the wrong resource. This is exactly
  the bug class handles exist to catch.

## Consequences

- Looking up a handle whose generation doesn't match the pool's current
  generation for that slot is a detectable, explicit error — not a
  dangling pointer or, worse, silent access to a *different*, reused
  resource.
- Resource pools decide reload/eviction policy in one place; nothing
  needs cooperative refcounting to know when a slot may be reused.
- Handles are trivially serializable (two `u32`s), which `Arc<T>` is not —
  relevant for save/replay and for editor tooling that needs to reference
  resources across a serialization boundary.
- Cost: callers must go through the owning pool to resolve a handle to
  actual data, rather than dereferencing directly. This is considered the
  correct tradeoff — see [ADR 003](003-no-global-managers.md) on why that
  pool is not a global singleton either.
