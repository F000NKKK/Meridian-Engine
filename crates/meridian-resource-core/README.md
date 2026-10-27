# meridian-resource-core

Resource identity: typed handles, versioning, lifetime and dependency tracking for textures/meshes/buffers/shaders. Not a manager — the type system resource lifecycle is built on.

**Layer and context:** shared kernel · identity

- **For:** Typed `ResourceId<T>`, `Version`, and a `DependencyGraph` with cycle checks.
- **Not for:** lifetime policy or loading: it is not a manager. Decoding is `meridian-asset-core`; storage is `meridian-memory-core`.

## Usage

```rust
use meridian_memory_core::ResourcePool;
use meridian_resource_core::{DependencyGraph, ResourceDependency, ResourceId};

let mut pool = ResourcePool::new();
let material = ResourceId::<u8>::new(pool.insert(0u8));
let texture = ResourceId::<u16>::new(pool.insert(1u8));

let mut graph = DependencyGraph::new();
graph.add_dependency(ResourceDependency::new(material, texture));
assert!(graph.depends_on(material.handle, texture.handle));
```

## Depends on / used by

- Depends on: [`meridian-memory-core`](../meridian-memory-core)
- Used by: [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-physics-core`](../meridian-physics-core)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
