# meridian-graphics-driver

Low-level GPU abstraction (device, command queues, buffers, textures, shaders, pipelines, synchronization). Knows nothing about scenes or materials.

**Layer and context:** infrastructure (adapter) · rendering

- **For:** The graphics device (headless or windowed), surfaces, render pipelines, vertex layouts and command recording.
- **Not for:** scenes, materials or the render graph: those are `meridian-graphics-core`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-graphics-driver); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-foundation`](../meridian-foundation), [`meridian-gpu-driver`](../meridian-gpu-driver), [`meridian-platform-core`](../meridian-platform-core)
- Used by: [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
