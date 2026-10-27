# meridian-graphics-core

High-level rendering domain: render graph, scene extraction, visibility/culling, lighting, materials, camera and the ports to a GPU.

**Layer and context:** domain · rendering

- **For:** `Scene3d`/`Scene2d`, `Camera`, lights and materials, the render graph, sprite batching, and the ports `FrameRecorder`, `MeshStore` and `TextureStore` (with fakes and contract suites under `testing`).
- **Not for:** owning GPU objects, pipelines or recording: those are `meridian-graphics-driver`, which implements the ports.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-graphics-core); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `testing` | `testing::FakeFrameRecorder`, `FakeTextureStore` and the `run_*_contract` suites |
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-asset-core`](../meridian-asset-core), [`meridian-ecs-core`](../meridian-ecs-core), [`meridian-foundation`](../meridian-foundation), [`meridian-gac-core`](../meridian-gac-core), [`meridian-memory-core`](../meridian-memory-core), [`meridian-resource-core`](../meridian-resource-core)
- Used by: [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-sdk`](../meridian-sdk), [`meridian-ui-render`](../meridian-ui-render)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
