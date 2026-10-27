# meridian-rendering-domain

High-level rendering domain: render graph, scene extraction, visibility/culling, lighting, materials, camera and the ports to a GPU.

**Layer and context:** domain · rendering

- **For:** `Scene3d`/`Scene2d`, `Camera`, lights and materials, the render graph, sprite batching, the mesh/material registries and the shadow light.
- **Not for:** owning GPU objects, pipelines or recording: those are `meridian-rendering-wgpu`, which implements the ports of `meridian-rendering-app`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-rendering-domain); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-ecs`](../meridian-ecs), [`meridian-diagnostics`](../meridian-diagnostics), [`meridian-gac`](../meridian-gac), [`meridian-memory`](../meridian-memory), [`meridian-resource`](../meridian-resource)
- Used by: [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-rendering-app`](../meridian-rendering-app), [`meridian-sdk`](../meridian-sdk), [`meridian-ui-scene2d`](../meridian-ui-scene2d)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
