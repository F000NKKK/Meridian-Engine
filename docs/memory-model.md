# Memory model — `meridian-memory-core`

Goal: minimize dynamic allocation in the hot path. Three allocation
strategies cover essentially everything the engine needs.

## Frame arena

Allocated at frame start, reset (not freed-and-reallocated) at frame end.
For any data whose lifetime is "created this frame, used this frame,
discarded before the next one" — command buffer scratch data, per-frame
culling results, transient render-graph resources.

## Persistent arena

For data that outlives a single frame but still follows a coarse,
predictable lifetime (level-lifetime data, loaded-scene data). Not freed
piecemeal; freed in bulk when its owning scope ends.

## Resource pools

For textures, meshes, buffers, and other GPU/CPU resources with irregular,
individually-tracked lifetimes. Backed by generational handles (below), not
by reference counting.

## Handles instead of `Arc<T>`

```rust
struct Handle {
    index: u32,
    generation: u32,
}
```

A `TextureHandle` is a plain, `Copy`, serializable value — not a smart
pointer. Looking a handle up against a stale generation is a detectable
error instead of a dangling reference; nothing needs to track a reference
count to know when a resource's slot may be reused. See
[ADR 002](adr/002-handle-based-resources.md) for the full rationale
(including why `Arc<Texture>` specifically was rejected).

The generic `Handle` above lives here; typed resource identity built on top
of it (`TextureHandle`, `MeshHandle`, versioning, cross-resource dependency
tracking) lives one layer up, in `meridian-resource-core` — see
[ADR 006](adr/006-resource-core-separation.md).

## Who owns lifetime decisions

`memory-core` provides the arenas, pools, and handle machinery — it does not
decide *when* something is freed. That decision belongs to whichever
higher-level crate holds the resource pool (consistent with
[ADR 003](adr/003-no-global-managers.md): there is no global
`ResourceManager` making that call on the engine's behalf).
