# meridian-audio-core

Spatial audio built on the GAC: mixer, DSP graph, listener and emitter frames.

**Layer and context:** domain · audio

- **For:** Spatial audio: `Listener`, `Emitter`, `Mixer`, `spatial_gains`, attenuation and a DSP node graph.
- **Not for:** device I/O: the `AudioSink` port is `meridian-audio-app`, and opening streams is `meridian-audio-driver`, which implements it.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-audio-core); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `testing` | `testing::RecordingSink` and `testing::run_contract` for adapters' tests |
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-gac-core`](../meridian-gac-core)
- Used by: [`meridian-audio-app`](../meridian-audio-app), [`meridian-audio-driver`](../meridian-audio-driver), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
