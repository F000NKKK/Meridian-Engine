# meridian-graphics-core

High-level rendering: render graph, scene extraction, visibility/culling, lighting, materials, camera and post-processing.

**Layer and context:** domain · rendering

- **For:** `Scene3d`/`Scene2d`, `Camera`, lights and materials, the render graph, bloom, and the submission bridge to `graphics-driver`.
- **Not for:** owning GPU objects or the device: those are `meridian-graphics-driver`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-graphics-core); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-asset-core`](../meridian-asset-core), [`meridian-ecs-core`](../meridian-ecs-core), [`meridian-foundation`](../meridian-foundation), [`meridian-gac-core`](../meridian-gac-core), [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-memory-core`](../meridian-memory-core), [`meridian-resource-core`](../meridian-resource-core)
- Used by: [`meridian-sdk`](../meridian-sdk), [`meridian-ui-render`](../meridian-ui-render)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
