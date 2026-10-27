# meridian-audio-domain

Spatial audio built on the GAC: mixer, DSP graph, listener and emitter frames.

**Layer and context:** domain · audio

- **For:** Spatial audio: `Listener`, `Emitter`, `Mixer`, `spatial_gains`, attenuation and a DSP node graph.
- **Not for:** device I/O: the `AudioSink` port is `meridian-audio-app`, and opening streams is `meridian-audio-cpal`, which implements it.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-audio-domain); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `testing` | `testing::RecordingSink` and `testing::run_contract` for adapters' tests |
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-gac`](../meridian-gac)
- Used by: [`meridian-audio-app`](../meridian-audio-app), [`meridian-audio-cpal`](../meridian-audio-cpal), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
