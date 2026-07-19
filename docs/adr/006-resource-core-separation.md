# ADR 006: A separate resource-core between memory and assets

## Status

Accepted

## Context

`meridian-memory-core` provides generic generational handles and pool
storage; `meridian-asset-core` loads and decodes files into CPU-side
representations. Neither owns the question of who defines a `TextureHandle`,
`MeshHandle`, `BufferHandle`, or `ShaderHandle` as a distinct, typed
resource identity — with its own versioning and cross-resource dependency
tracking (a material referencing a texture, a mesh referencing a skeleton).
Without an explicit answer, each subsystem crate would end up defining its
own ad hoc handle newtype, re-deriving the same generational-index pattern
with no shared versioning or dependency-tracking behavior.

## Decision

Add `meridian-resource-core`, depending only on `meridian-memory-core`.
It defines resource identity — `Handle`-based `ResourceId` types,
versioning, and dependency tracking between resources — and nothing else.
`graphics-core`, `physics-core`, and `audio-core` depend on it for their
respective handle types (`TextureHandle`, `MeshHandle`, `BufferHandle`,
`ShaderHandle`, collider mesh handles, and so on).

`meridian-asset-core` deliberately does **not** depend on it: asset-core's
job ends at "file bytes → decoder → CPU-side representation" (see
[ADR 003](003-no-global-managers.md) and
[dependency-rules.md](../dependency-rules.md) rule 4); resource identity
and lifetime are concerns of the crate that actually holds the resource
pool, not of the loader.

## Alternatives considered

- **Fold this into `memory-core`** — rejected because `memory-core`'s
  `Handle` is a generic, domain-agnostic generational index; resource
  *identity* (what a `TextureHandle` specifically means, its version, what
  it depends on) is a step up in specificity that doesn't belong in the
  same crate as raw arena/pool mechanics.
- **Fold this into `asset-core`** — rejected because asset-core explicitly
  does not manage ownership or lifetime (its own founding constraint); a
  resource-identity/dependency-tracking type is exactly the kind of thing
  that constraint rules out.
- **Let each `*-core` define its own handle type independently** — the
  status quo before this ADR. Rejected because it means re-deriving
  versioning and dependency-tracking three times with no shared contract,
  and no single place to reason about "what does this handle depend on."

## Consequences

- `graphics-core`, `physics-core`, and `audio-core` share one resource-
  identity model instead of three divergent ones.
- `resource-core` must resist scope creep toward becoming a
  `ResourceManager` — it is a type system, not a runtime service. If a PR
  adds loading, caching, or eviction logic to this crate, that's a
  violation of rule 8 in [dependency-rules.md](../dependency-rules.md).
