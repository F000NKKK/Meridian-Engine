# meridian-audio-core

Spatial audio built on the GAC: mixer, DSP graph, listener and emitter frames.

**Layer and context:** domain · audio

- **For:** Spatial audio: `Listener`, `Emitter`, `Mixer`, `spatial_gains`, attenuation and a DSP node graph.
- **Not for:** device I/O: opening streams is `meridian-audio-driver`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-audio-core); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-audio-driver`](../meridian-audio-driver), [`meridian-gac-core`](../meridian-gac-core), [`meridian-resource-core`](../meridian-resource-core)
- Used by: [`meridian-engine-core`](../meridian-engine-core), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
