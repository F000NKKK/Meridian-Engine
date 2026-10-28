# meridian-resource

Resource identity: typed handles, versioning, lifetime and dependency tracking for textures/meshes/buffers/shaders. Not a manager — the type system resource lifecycle is built on.

**Layer and context:** shared kernel · identity

- **For:** Typed `ResourceId<T>`, `Version`, a `DependencyGraph` with cycle checks, and `ResourceIdTable<K, T>` (a stable key such as an asset id to its runtime `ResourceId`, bound by the code that loads and unbound by the code that releases).
- **Not for:** lifetime policy or loading: it is not a manager. Decoding is `meridian-asset-domain`; storage is `meridian-memory`.

## Usage

```rust
use meridian_memory::ResourcePool;
use meridian_resource::{DependencyGraph, ResourceDependency, ResourceId};

let mut pool = ResourcePool::new();
let material = ResourceId::<u8>::new(pool.insert(0u8));
let texture = ResourceId::<u16>::new(pool.insert(1u8));

let mut graph = DependencyGraph::new();
graph.add_dependency(ResourceDependency::new(material, texture));
assert!(graph.depends_on(material.handle, texture.handle));
```

## Depends on / used by

- Depends on: [`meridian-memory`](../meridian-memory)
- Used by: [`meridian-rendering-domain`](../meridian-rendering-domain), [`meridian-physics-domain`](../meridian-physics-domain)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
