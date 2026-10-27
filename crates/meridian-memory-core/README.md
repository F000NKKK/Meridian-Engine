# meridian-memory-core

Frame/persistent arenas, resource pools and generational handles; minimizes dynamic allocation across the engine.

**Layer and context:** shared kernel · identity

- **For:** `ResourcePool<T>` with generational `Handle`s, `FrameArena` and `PersistentArena`.
- **Not for:** typed resource identity, versions and dependency tracking: those are `meridian-resource-core`.

## Usage

```rust
use meridian_memory_core::ResourcePool;

let mut pool = ResourcePool::new();
let handle = pool.insert("mesh");
assert_eq!(pool.get(handle), Some(&"mesh"));
pool.remove(handle);
// The slot's generation moved on: the stale handle no longer resolves.
assert_eq!(pool.get(handle), None);
```

## Depends on / used by

- Depends on: no workspace crate
- Used by: [`meridian-ecs-core`](../meridian-ecs-core), [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-resource-core`](../meridian-resource-core), [`meridian-ui-core`](../meridian-ui-core)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
