# meridian-rendering-app

Rendering use cases and the ports to the GPU: `FrameRecorder`, `TextureStore` and `MeshStore`.

**Layer and context:** application · rendering

- **For:** the ports `FrameRecorder`, `TextureStore` and `MeshStore` (declared by the layer that needs them, ADR 019), `MeshRegistry` as a `MeshStore`, and fakes plus contract suites under the `testing` feature.
- **Not for:** the scene, materials, meshes or handles: those are `meridian-rendering-domain`. Pipelines and recording are `meridian-rendering-wgpu`, which implements the ports.

## Usage

```rust
use meridian_rendering_domain::{MeshRegistry, MeshSource};
use meridian_rendering_app::MeshStore;

let mut meshes = MeshRegistry::new();
let triangle = MeshSource {
    positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
    normals: vec![[0.0, 0.0, 1.0]; 3],
    uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
    indices: vec![0, 1, 2],
};
let handle = meshes.add(triangle).expect("a valid triangle");
assert!(meshes.get(handle).is_some());
```

## Features

| Feature | Enables |
|---|---|
| `testing` | `testing::FakeFrameRecorder`, `FakeTextureStore` and the `run_*_contract` suites |

## Depends on / used by

- Depends on: [`meridian-asset-domain`](../meridian-asset-domain), [`meridian-rendering-domain`](../meridian-rendering-domain), [`meridian-memory`](../meridian-memory)
- Used by: [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
