# meridian-audio-driver

Low-level audio device abstraction: output streams, buffers and platform audio backends.

**Layer and context:** infrastructure (adapter) · audio

- **For:** The audio output device (via `cpal`): streams and ring buffers.
- **For:** implementing `meridian_audio_core::AudioSink` with `AudioStream`; `open_output` opens one for a `SpeakerLayout`.
- **Not for:** mixing, DSP or spatialization: those are `meridian-audio-core`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-audio-driver); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-audio-core`](../meridian-audio-core), [`meridian-foundation`](../meridian-foundation), [`meridian-platform-core`](../meridian-platform-core)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
