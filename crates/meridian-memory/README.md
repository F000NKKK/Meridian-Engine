# meridian-memory

Frame/persistent arenas, resource pools and generational handles; minimizes dynamic allocation across the engine.

**Layer and context:** shared kernel · identity

- **For:** `ResourcePool<T>` with generational `Handle`s, `FrameArena` and `PersistentArena`.
- **Not for:** typed resource identity, versions and dependency tracking: those are `meridian-resource`.

## Usage

```rust
use meridian_memory::ResourcePool;

let mut pool = ResourcePool::new();
let handle = pool.insert("mesh");
assert_eq!(pool.get(handle), Some(&"mesh"));
pool.remove(handle);
// The slot's generation moved on: the stale handle no longer resolves.
assert_eq!(pool.get(handle), None);
```

## Depends on / used by

- Depends on: no workspace crate
- Used by: [`meridian-ecs`](../meridian-ecs), [`meridian-rendering-domain`](../meridian-rendering-domain), [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-rendering-app`](../meridian-rendering-app), [`meridian-resource`](../meridian-resource), [`meridian-ui-domain`](../meridian-ui-domain)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
